//! Regenerate `apps/desktop/src/bindings.ts` and the registry test fixture
//! (`apps/desktop/src/test/fixtures/registry.json`):
//! `cargo run -p katabox-arch-desktop --features bindings --bin export-bindings`.

fn main() {
    katabox_arch_lib::export_bindings();
    katabox_arch_lib::export_registry_fixture();
    katabox_arch_lib::export_frame_fixture();
    println!(
        "wrote apps/desktop/src/bindings.ts, src/test/fixtures/registry.json and src/test/fixtures/frame.kaf"
    );
}
