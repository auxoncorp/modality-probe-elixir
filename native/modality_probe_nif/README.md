# NIF for Elixir.ModalityProbe

## To build the NIF module:

- Your NIF will now build along with your project.

## To load the NIF:

```elixir
defmodule use Rustler, otp_app: :modality_probe_elixir, crate: "modalityprobe"

    # When your NIF is loaded, it will override this function.
    def add(_a, _b), do: :erlang.nif_error(:nif_not_loaded)
end
```

## Examples

[This](https://github.com/hansihe/NifIo) is a complete example of a NIF written in Rust.
