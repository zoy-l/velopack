import os
import subprocess
import sys
import shutil

def run_command(command, cwd=None):
    print(f"Running: {' '.join(command)}")
    result = subprocess.run(command, cwd=cwd, shell=False)
    if result.returncode != 0:
        print(f"Error: Command failed with exit code {result.returncode}")
        sys.exit(result.returncode)

def main():
    # Since this script is now in 'scripts/' directory, root is one level up
    root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    # 1. Prepare Rust environment
    print("--- Preparing Rust Environment ---")
    # rust-src is required for -Z build-std
    run_command(["rustup", "component", "add", "rust-src", "--toolchain", "nightly"])

    # 2. Build Rust binaries
    print("\n--- Building Rust Binaries ---")
    # We must enable the 'windows' feature to build setup.exe and stub.exe.
    # We also use -Z build-std to minimize binary size for Update.exe.
    cargo_cmd = [
        "cargo", "+nightly", "build",
        "--release",
        "-Z", "build-std=core,alloc,std,panic_abort",
        "-p", "velopack_bins"
    ]
    if sys.platform == "win32":
        cargo_cmd.insert(4, "--features")
        cargo_cmd.insert(5, "windows")
    run_command(cargo_cmd, cwd=root_dir)

    # 3. Create dummy files for cross-platform binaries (if missing)
    # The .NET build requires these files to exist when PackRustAssets=true.
    print("\n--- Creating Binaries Placeholders ---")
    target_release = os.path.join(root_dir, "target", "release")
    os.makedirs(target_release, exist_ok=True)

    if sys.platform == "darwin":
        shutil.copy(os.path.join(target_release, "update"), os.path.join(target_release, "UpdateMac"))
    elif sys.platform == "linux":
        # we only support x64/arm64 linux for now
        import platform
        arch = platform.machine().lower()
        if "arm" in arch or "aarch" in arch:
            shutil.copy(os.path.join(target_release, "update"), os.path.join(target_release, "UpdateNix_arm64"))
        else:
            shutil.copy(os.path.join(target_release, "update"), os.path.join(target_release, "UpdateNix_x64"))

    placeholders = ["UpdateMac", "UpdateNix_x64", "UpdateNix_arm64", "update.exe", "setup.exe", "stub.exe"]
    for p in placeholders:
        p_path = os.path.join(target_release, p)
        if not os.path.exists(p_path):
            print(f"Creating placeholder: {p}")
            with open(p_path, "w") as f:
                f.write("placeholder")

    # 4. Build .NET CLI
    print("\n--- Building Velopack Vpk CLI ---")
    vpk_proj = os.path.join(root_dir, "src", "vpk", "Velopack.Vpk", "Velopack.Vpk.csproj")
    dotnet_cmd = [
        "dotnet", "build", vpk_proj,
        "-c", "Release",
        "/p:PackRustAssets=true"
    ]
    run_command(dotnet_cmd, cwd=root_dir)

    # 3. Locating output
    print("\n--- Build Success ---")
    # Output usually goes to build/Release/netX.0/vpk.exe based on Directory.Build.props
    output_dir = os.path.join(root_dir, "build", "Release")
    vpk_name = "vpk.exe" if sys.platform == "win32" else "vpk"
    if os.path.exists(output_dir):
        print(f"Build artifacts are located in: {output_dir}")
        for root, dirs, files in os.walk(output_dir):
            if vpk_name in files:
                print(f"Found CLI at: {os.path.join(root, vpk_name)}")
    else:
        print("Warning: Could not find build output directory. Please check the logs above.")

if __name__ == "__main__":
    main()
