# ModalityProbe

An Elixir API for https://github.com/auxoncorp/modality-probe

## Installation

Add this to your `mix.exs`:


```elixir
def deps do
  [
    {:modality_probe, git: "https://github.com/auxoncorp/modality-probe-elixir.git" }
  ]
end
```

## Usage

```elixir
  probe = Elixir.ModalityProbe.create(42, 1024, 0, nil, nil, save_restart_counter_pid)
  Elixir.ModalityProbe.record_event(probe, 10)
```

## TODO
- Code scanning / manifest generation
- Mutation support
