defmodule ModalityProbe do
  use Rustler, otp_app: :modality_probe, crate: "modality_probe_nif"

  @doc """
  Create a new Modality probe. 

  - `probe_id`: The 32 bit ID of this probe; must be unique across the system. 

  - `buffer_size`: The size in bytes of the buffer to allocate for the probe.
     This is allocated on the process heap, in memory not managed by the VM.

  - `initial_restart_counter`: The current value of the persistent restart counter for this probe.

  - `time_resolution` (Optional): The minimum distinguashable timespan of this probe's clock, in ns.

  - `wall_clock_id` (Optional): The system-level id of this probe's clock, if it is shared with other probes.
     If not given, all times are considered to be local to the probe.

  - `restart_counter_notify` (Optional): A pid to which a `%{probe_id: x, restart_counter: y}` message will be sent
     every time the restart counter is ticked. This will always happen on probe creation, and may also happen periodically
     when using other api calls due to overflow of inner clock segment. 
  """
  def create(_probe_id, _buffer_size, _initial_restart_counter, _time_resolution = nil, _wall_clock_id = nil, _restart_counter_notify = nil), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Get the current value of the restart counter.

  This can be called if you want to directly manage persistence of the
  restart counter, perhaps by wrapping this api to check it after every call.
  """
  def get_restart_counter(_probe), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Record a timestamp into the log.
  """
  def record_time(_probe, _time), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Record an event.
  """
  def record_event(_probe, _event), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Record an event, along with a timestamp.
  """
  def record_event_with_time(_probe, _event, _time), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Record an event, with an arbitrary 32 bit payload.
  """
  def record_event_with_payload(_probe, _event, _payload), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Record an event, with an arbitrary 32 bit payload, along with a timestamp.
  """
  def record_event_with_payload_with_time(_probe, _event, _payload, _time), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Produce a snapshot, as a Binary.
  """
  def produce_snapshot_bytes(_probe), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Produce a snapshot, as a Binary, logging time.
  """
  def produce_snapshot_bytes_with_time(_probe, _time), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Merge a snapshot, given as a Binary.
  """
  def merge_snapshot_bytes(_probe, _snapshot), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Merge a snapshot, given as a Binary, logging time..
  """
  def merge_snapshot_bytes_with_time(_probe, _snapshot, _time), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Produce a report, as a Binary.

  `report_size` gives size in bytes of the Binary that will be allocated for the report. 

  Returns:
  - `nil`: if there are not new events to send, since the last time `report` was called
  - `{byte_size, binary}`: `byte_size` is the number of bytes in the `binary` that
     actually have report data; only this much data needs to be sent to modalityd.
  """
  def report(_probe, _report_size), do: :erlang.nif_error(:nif_not_loaded)
end
