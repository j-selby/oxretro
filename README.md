oxretro
=======

An experimental implementation of the LibRetro API with a multi-process model.

Building
--------

You are going to need Rust Nightly.

```bash
cargo build --release
```

Final binary will be at `target/release/oxretro[.exe]`.

Running
-------

To run the frontend + backend together:

```bash
oxretro --core=path/to/core[.dll,.so,.dylib] --rom=path/to/rom.[whatever]
```

To run them separately:

- On the frontend:
```bash
oxretro --type=frontend --address=127.0.0.1:1234 --rom=path/to/rom.[whatever] --no-backend
```

- On the backend:
```bash
oxretro --type=backend --address=127.0.0.1:1234 --core=path/to/core[.dll,.so,.dylib] 
```

License
-------

oxretro is licensed to you under the Apache 2.0 license, which can be found [here](LICENSE).