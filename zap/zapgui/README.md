# egui_template

## new.sh

makes a copy of this template next to it with a name you type.
skips target, .git, Cargo.lock.

./new.sh

## local_cargo.sh

I have nightly with cranelift and other stuff that egui doesn't work with.
I don't want to change my global settings, so I run this instead for building.

If I'm in the project root, I don't need to do anything different — my `c`
alias checks for local_cargo.sh in the current folder and uses it if it's
there. Otherwise it falls back to plain `cargo`. So `c run`, `c build`, etc
just work.

If I'm not in the root, use the file directly:

./local_cargo.sh                # run
./local_cargo.sh build
./local_cargo.sh build -p core
./local_cargo.sh add serde
./local_cargo.sh clean
