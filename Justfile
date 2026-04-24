build:
    tailwindcss -i src/input.css -o static/output.css --minify

watch:
    tailwindcss -i src/input.css -o static/output.css --watch

run:
    cargo run

dev:
    just watch &
    cargo watch -x run
