use std::{
    convert::TryInto,
    mem::MaybeUninit,
    pin::Pin,
    sync::{
        atomic::{AtomicU16, Ordering},
        Mutex, MutexGuard,
    },
};

use convert::produce_error;
use modality_probe::{
    CRestartCounterProvider, ModalityProbe, NanosecondResolution, Probe, ProbeId, WallClockId,
};
use rustler::{Encoder, ResourceArc};

rustler::init!(
    "Elixir.ModalityProbe",
    [
        create,
        get_restart_counter,
        record_time,
        record_event,
        record_event_with_time,
        record_event_with_payload,
        record_event_with_payload_with_time,
        produce_snapshot_bytes,
        produce_snapshot_bytes_with_time,
        merge_snapshot_bytes,
        merge_snapshot_bytes_with_time,
        report
    ],
    load = load
);

fn load(env: rustler::Env, _: rustler::Term) -> bool {
    rustler::resource!(ElixirProbe, env);
    true
}

pub struct ElixirProbe {
    probe_id: u32,

    // Give this a (false) static lifetime, since it really refers to `data` below.
    probe: Mutex<ModalityProbe<'static>>,

    // This is pinned on the heap because `probe.restart_counter` holds a pointer to it
    restart_counter_state: Pin<Box<AtomicU16>>,

    // this is pinned because `probe` holds references to it
    #[allow(unused)]
    data: Pin<Box<[MaybeUninit<u8>]>>,

    restart_counter_notify: Option<rustler::types::pid::Pid>,
}

impl ElixirProbe {
    /// Acquire the probe mutex, then do a thing with it. Pay attention to the value of the restart
    /// counter while we're at it; if it moves, send off a message to the configured pid.
    fn with_locked_probe<F: FnMut(&mut ModalityProbe) -> Result<O, rustler::Error>, O>(
        &self,
        env: rustler::Env,
        mut f: F,
    ) -> Result<O, rustler::Error> {
        let mut probe: MutexGuard<_> = self
            .probe
            .lock()
            .map_err(|_| rustler::Error::RaiseAtom("probe_mutex_poisoned"))?;

        let restart_counter_before = self.restart_counter_state.load(Ordering::SeqCst);
        let res = f(&mut probe);
        let restart_counter_after = self.restart_counter_state.load(Ordering::SeqCst);

        if restart_counter_before != restart_counter_after {
            self.send_restart_counter_notification(env)?;
        }

        res
    }

    fn send_restart_counter_notification(&self, env: rustler::Env) -> Result<(), rustler::Error> {
        if let Some(pid) = &self.restart_counter_notify {
            let probe_id_key = rustler::types::atom::Atom::from_str(env, "probe_id")?;
            let restart_counter_key = rustler::types::atom::Atom::from_str(env, "restart_counter")?;
            let message = rustler::Term::map_new(env)
                .map_put(probe_id_key.encode(env), self.probe_id.encode(env))?
                .map_put(
                    restart_counter_key.encode(env),
                    self.restart_counter_state
                        .load(Ordering::SeqCst)
                        .encode(env),
                )?;
            env.send(pid, message);
        }

        Ok(())
    }
}

/// SAFETY: ProbeInner is not Send+Sync because ModalityProbe is not Send, which is because of:
///  - CRestartCounterProvider, but that's an enum variant we're not using
///  - RustRestartCounterProvider, which we are using. But our implementation
///    `MemRestartCounter` is in fact Send+Sync, since it's just a newtype on an atomic.
unsafe impl Send for ElixirProbe {}
unsafe impl Sync for ElixirProbe {}

extern "C" fn next_sequence_id(
    _probe_id: u32,
    state: *mut core::ffi::c_void,
    out_sequence_id: *mut u16,
) -> usize {
    let state: &AtomicU16 = unsafe { &*(state as *const std::sync::atomic::AtomicU16) };

    // This operation wraps around on overflow.
    let old = state.fetch_add(1, Ordering::SeqCst);
    let new = old.wrapping_add(1);

    unsafe {
        *out_sequence_id = new;
    }

    0
}

#[rustler::nif]
pub fn create(
    env: rustler::Env,
    probe_id: u32,
    buffer_size: usize,
    initial_restart_counter: u16,
    time_resolution: Option<u32>,
    wall_clock_id: Option<u16>,
    restart_counter_notify: Option<rustler::types::Pid>,
) -> Result<ResourceArc<ElixirProbe>, rustler::Error> {
    let typed_probe_id: ProbeId = probe_id
        .try_into()
        .map_err(|_| rustler::Error::RaiseAtom("invalid_probe_id"))?;

    let time_resolution = match time_resolution {
        Some(x) => x.into(),
        None => NanosecondResolution::UNSPECIFIED,
    };

    let wall_clock_id = match wall_clock_id {
        Some(x) => x.into(),
        None => WallClockId::LOCAL_ONLY,
    };

    /////////////////////
    // Restart Counter //
    /////////////////////

    let restart_counter_state = Box::pin(AtomicU16::new(initial_restart_counter));

    let restart_counter = Some(CRestartCounterProvider {
        iface: next_sequence_id,
        // SAFETY: `std::mem::transmute` into `void *` is safe here
        // because this is only read by `next_sequence_id` above,
        // which transmutes it back into `&AtomicU16`.
        state: unsafe { std::mem::transmute(&*restart_counter_state) },
    });

    ////////////////
    // Data Slice //
    ////////////////

    let mut data = Pin::new(vec![MaybeUninit::new(0u8); buffer_size].into_boxed_slice());

    // SAFETY: `std::mem::transmute` into the static lifetime is safe here because we're the
    // reference is being handed to ModalityProbe::initialize, which puts it inside of the data
    // buffer, whose lifetime is going to be the same as restart_counter because they're both going
    // inside of ElixirProbe.
    let pinned_data_ref: Pin<&'static mut [MaybeUninit<u8>]> =
        unsafe { std::mem::transmute(data.as_mut()) };

    // SAFETY: `get_unchecked_mut` is safe here because `new_with_storage` does not move anything out
    // of the reference.
    let data_ref: &'static mut [MaybeUninit<u8>] =
        unsafe { Pin::get_unchecked_mut(pinned_data_ref) };

    let probe = ModalityProbe::new_with_storage(
        data_ref,
        typed_probe_id,
        time_resolution,
        wall_clock_id,
        restart_counter,
    )
    .map_err(convert::storage_setup_error)?;

    let ep = ElixirProbe {
        probe_id,
        probe: Mutex::new(probe),
        data,
        restart_counter_state,
        restart_counter_notify,
    };

    // The probe ticks the clock once on startup, so send the notification
    ep.send_restart_counter_notification(env)?;

    Ok(ResourceArc::new(ep))
}

#[rustler::nif]
pub fn get_restart_counter(probe: ResourceArc<ElixirProbe>) -> u16 {
    probe.restart_counter_state.load(Ordering::SeqCst)
}

#[rustler::nif]
pub fn record_time(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    time: u64,
) -> Result<(), rustler::Error> {
    let time = convert::time(time)?;
    elixir_probe.with_locked_probe(env, |probe| {
        probe.record_time(time);
        Ok(())
    })
}

#[rustler::nif]
pub fn record_event(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    event_id: u32,
) -> Result<(), rustler::Error> {
    let event_id = convert::event_id(event_id)?;

    elixir_probe.with_locked_probe(env, |probe| {
        probe.record_event(event_id);
        Ok(())
    })
}

#[rustler::nif]
pub fn record_event_with_time(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    event_id: u32,
    time: u64,
) -> Result<(), rustler::Error> {
    let event_id = convert::event_id(event_id)?;
    let time = convert::time(time)?;

    elixir_probe.with_locked_probe(env, |probe| {
        probe.record_event_with_time(event_id, time);
        Ok(())
    })
}

#[rustler::nif]
pub fn record_event_with_payload(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    event_id: u32,
    payload: u32,
) -> Result<(), rustler::Error> {
    let event_id = convert::event_id(event_id)?;

    elixir_probe.with_locked_probe(env, |probe| {
        probe.record_event_with_payload(event_id, payload);
        Ok(())
    })
}

#[rustler::nif]
pub fn record_event_with_payload_with_time(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    event_id: u32,
    payload: u32,
    time: u64,
) -> Result<(), rustler::Error> {
    let event_id = convert::event_id(event_id)?;
    let time = convert::time(time)?;

    elixir_probe.with_locked_probe(env, |probe| {
        probe.record_event_with_payload_with_time(event_id, payload, time);
        Ok(())
    })
}

#[rustler::nif]
pub fn produce_snapshot_bytes(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
) -> Result<rustler::types::binary::Binary, rustler::Error> {
    elixir_probe.with_locked_probe(env, move |probe| {
        let mut snapshot_bytes = rustler::types::binary::OwnedBinary::new(12).ok_or(
            rustler::error::Error::RaiseAtom("alloc_probe_snapshot_failure"),
        )?;

        let written_byte_count = probe
            .produce_snapshot_bytes(snapshot_bytes.as_mut_slice())
            .map_err(produce_error)?;
        if written_byte_count != 12 {
            return Err(rustler::Error::RaiseAtom("produce_snapshot_bad_data_size"));
        }

        Ok(rustler::types::binary::Binary::from_owned(
            snapshot_bytes,
            env,
        ))
    })
}

#[rustler::nif]
pub fn produce_snapshot_bytes_with_time(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    time: u64,
) -> Result<rustler::types::binary::Binary, rustler::Error> {
    let time = convert::time(time)?;

    elixir_probe.with_locked_probe(env, move |probe| {
        let mut snapshot_bytes = rustler::types::binary::OwnedBinary::new(12).ok_or(
            rustler::error::Error::RaiseAtom("alloc_probe_snapshot_failure"),
        )?;

        let written_byte_count = probe
            .produce_snapshot_bytes_with_time(time, snapshot_bytes.as_mut_slice())
            .map_err(convert::produce_error)?;
        if written_byte_count != 12 {
            return Err(rustler::Error::RaiseAtom("produce_snapshot_bad_data_size"));
        }

        Ok(rustler::types::binary::Binary::from_owned(
            snapshot_bytes,
            env,
        ))
    })
}

#[rustler::nif]
pub fn merge_snapshot_bytes(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    snapshot_bytes: rustler::types::binary::Binary,
) -> Result<(), rustler::Error> {
    elixir_probe.with_locked_probe(env, move |probe| {
        probe
            .merge_snapshot_bytes(snapshot_bytes.as_slice())
            .map_err(convert::merge_error)
    })
}

#[rustler::nif]
pub fn merge_snapshot_bytes_with_time(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    snapshot_bytes: rustler::types::binary::Binary,
    time: u64,
) -> Result<(), rustler::Error> {
    let time = convert::time(time)?;

    elixir_probe.with_locked_probe(env, move |probe| {
        probe
            .merge_snapshot_bytes_with_time(snapshot_bytes.as_slice(), time)
            .map_err(convert::merge_error)
    })
}

#[rustler::nif]
pub fn report(
    env: rustler::Env,
    elixir_probe: ResourceArc<ElixirProbe>,
    report_size: usize,
) -> Result<Option<(usize, rustler::types::binary::Binary)>, rustler::Error> {
    elixir_probe.with_locked_probe(env, move |probe| {
        let mut report_bytes = rustler::types::binary::OwnedBinary::new(report_size).ok_or(
            rustler::error::Error::RaiseAtom("alloc_report_buffer_failure"),
        )?;

        let maybe_written_bytes = probe
            .report(report_bytes.as_mut_slice())
            .map_err(convert::report_error)?;

        match maybe_written_bytes {
            Some(x) => {
                let out_bin = rustler::types::binary::Binary::from_owned(report_bytes, env);
                Ok(Some((x.into(), out_bin)))
            }
            None => Ok(None),
        }
    })
}

mod convert {
    use modality_probe::{
        EventId, MergeError, Nanoseconds, ProduceError, ReportError, StorageSetupError,
    };

    pub fn event_id(eid: u32) -> Result<EventId, rustler::Error> {
        EventId::new(eid).ok_or(rustler::error::Error::RaiseAtom("invalid_event_id"))
    }

    pub fn time(ns: u64) -> Result<Nanoseconds, rustler::Error> {
        Nanoseconds::new(ns).ok_or(rustler::error::Error::RaiseAtom("time_out_of_range"))
    }

    pub fn storage_setup_error(e: StorageSetupError) -> rustler::Error {
        let s = match e {
            StorageSetupError::UnderMinimumAllowedSize => "storage_under_minimum_allowed_size",
            StorageSetupError::ExceededMaximumAddressableSize => {
                "storage_exceeded_maximum_addressable_size"
            }
            StorageSetupError::NullDestination => "storage_null_destination",
        };

        rustler::error::Error::RaiseAtom(s)
    }
    pub fn produce_error(e: ProduceError) -> rustler::Error {
        let s = match e {
            ProduceError::InsufficientDestinationSize => "produce_insufficient_destination_size",
        };

        rustler::error::Error::RaiseAtom(s)
    }

    pub fn merge_error(e: MergeError) -> rustler::Error {
        let s = match e {
            MergeError::ExceededAvailableClocks => "merge_exceeded_available_clocks",
            MergeError::InsufficientSourceSize => "merge_insufficient_source_size",
            MergeError::ExternalHistorySemantics => "merge_external_history_semantic_violation",
        };

        rustler::error::Error::RaiseAtom(s)
    }

    pub fn report_error(e: ReportError) -> rustler::Error {
        let s = match e {
            ReportError::InsufficientDestinationSize => "report_insufficient_destination_size",
        };

        rustler::error::Error::RaiseAtom(s)
    }
}
