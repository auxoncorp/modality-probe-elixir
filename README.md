# ModalityProbe

An Elixir API for https://github.com/auxoncorp/modality-probe

## Dependencies

- Elixir
- A working Rust toolchain on your path. Get it from https://rustup.rs/
- An installation of modality, with the Rust SDK installed in its
  default location at `/usr/share/modality/modality-probe-sys`.  Get
  it from https://auxon.io/request-trial.

## Installation

Add this to your `mix.exs`:


```elixir
def deps do
  [
    { :modality_probe, git: "https://github.com/auxoncorp/modality-probe-elixir.git" }
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

## LICENSE 

See [LICENSE](./LICENSE) for more details.

Copyright 2020 [Auxon Corporation](https://auxon.io)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

[http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
