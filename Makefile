test:
    cargo test --no-fail-fast -- --nocapture 
db:
    docker-compose up -d
env:
    cp .env.example .env
    cp .envrc.example .envrc
doc:
    cargo doc --no-deps --open
