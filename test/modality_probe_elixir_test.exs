defmodule ModalityProbeTest do
  use ExUnit.Case
  doctest ModalityProbe

  test "smoke test" do
    defmodule SaveRestartCounterPid do
      def run do
        receive do
          %{probe_id: p, restart_counter: r} ->
            IO.inspect({p, r}, label: "Save this restart counter")
        end

        run()
      end
    end

    save_restart_counter_pid = spawn(SaveRestartCounterPid, :run, [])
    
    probe = Elixir.ModalityProbe.create(42, 1024, 0, nil, nil, save_restart_counter_pid)
    IO.inspect(probe, label: "Created probe")
    
    Elixir.ModalityProbe.record_event(probe, 10)
    Elixir.ModalityProbe.record_event(probe, 11)
    Elixir.ModalityProbe.record_event(probe, 12)
    
    ss = Elixir.ModalityProbe.produce_snapshot_bytes(probe)
    IO.inspect(ss, label: "Produced snapshot")
        
    {report_size, report_bytes} = Elixir.ModalityProbe.report(probe, 1024)
    IO.inspect(report_size, label: "Produced report size")
    IO.inspect(report_bytes, label: "Produced report bytes")
  end

end
