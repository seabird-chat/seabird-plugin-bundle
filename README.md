# seabird-rs

## Development

### Windows Setup for PostgreSQL (Without Docker)

1. Install `vcpkg`: https://github.com/microsoft/vcpkg#quick-start

2. **Note: make sure that you hook up user-wide integration.**

3. Install `vcpkg_cli`: `cargo install vcpkg_cli`.

4. Set the `VCPKGRS_DYNAMIC` environment variable to `1`. In PowerShell: `$Env:VCPKGRS_DYNAMIC = 1`.

5. Install `libpq`: `vcpkg install libpq:x64-windows`.

6. Ensure `libpq` is available: `vcpkg_cli probe libpq`.

7. Add the DLL bin directory (something like `$HOME\vcpkg\isntalled\x64-windows\bin`) to your **system** `Path` environment variable.

8. Remove any previously-built `pq` bindings: `cargo clean -p pq-sys`.

9. Build!
