import os
import subprocess
import sys

def run_command(command, cwd=None):
    print(f"Running: {' '.join(command)}")
    result = subprocess.run(command, cwd=cwd, shell=False)
    if result.returncode != 0:
        print(f"Error: Command failed with exit code {result.returncode}")
        sys.exit(result.returncode)

def check_and_install_tools():
    print("--- Checking Dependencies ---")
    # check if dotnet-coverage is installed
    result = subprocess.run(["dotnet", "tool", "list", "-g"], capture_output=True, text=True, shell=False)
    if "dotnet-coverage" not in result.stdout:
        print("Installing missing tool: dotnet-coverage")
        # We don't use run_command here to avoid exiting if installation fails
        subprocess.run(["dotnet", "tool", "install", "-g", "dotnet-coverage"], shell=False)
    else:
        print("dotnet-coverage is already installed.")

def ensure_placeholders(root_dir, config):
    print(f"--- Ensuring Placeholders for {config} ---")
    target_dir = os.path.join(root_dir, "target", config.lower())
    if not os.path.exists(target_dir):
        os.makedirs(target_dir)

    import shutil
    if sys.platform == "darwin":
        update_path = os.path.join(target_dir, "update")
        if os.path.exists(update_path):
            shutil.copy(update_path, os.path.join(target_dir, "UpdateMac"))
    elif sys.platform == "linux":
        update_path = os.path.join(target_dir, "update")
        if os.path.exists(update_path):
            import platform
            arch = platform.machine().lower()
            if "arm" in arch or "aarch" in arch:
                shutil.copy(update_path, os.path.join(target_dir, "UpdateNix_arm64"))
            else:
                shutil.copy(update_path, os.path.join(target_dir, "UpdateNix_x64"))

    # HelperFile.cs looks for "update" in DEBUG, and specific names in RELEASE
    # We create all of them to be safe.
    placeholders = ["update", "UpdateMac", "UpdateNix_x64", "UpdateNix_arm64", "update.exe", "setup.exe", "stub.exe"]
    for p in placeholders:
        p_path = os.path.join(target_dir, p)
        if not os.path.exists(p_path):
            print(f"Creating placeholder: {p}")
            with open(p_path, "w") as f:
                f.write("")

def main():
    # Since this script is now in 'scripts/' directory, root is one level up
    root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    # 0. Check and install required .NET tools
    check_and_install_tools()

    # 1. Determine configuration
    config = "Debug"
    if "--release" in sys.argv:
        config = "Release"

    print(f"Configuration: {config}")

    # 2. Ensure placeholders exist for tests
    ensure_placeholders(root_dir, config)

    # 3. Run Rust Tests
    print("\n--- Running Rust Tests ---")
    rust_bins_dir = os.path.join(root_dir, "src", "bins")
    if os.path.exists(rust_bins_dir):
        # Using --all-features to ensure all code paths are tested
        test_cmd = ["cargo", "test"]
        if sys.platform == "win32":
            test_cmd.append("--all-features")
        run_command(test_cmd, cwd=rust_bins_dir)
    else:
        print(f"Warning: Rust bins directory not found at {rust_bins_dir}. Skipping.")

    # 4. Run .NET Tests
    print("\n--- Running .NET Tests ---")
    sln_path = os.path.join(root_dir, "Velopack.sln")

    if os.path.exists(sln_path):
        run_command(["dotnet", "test", sln_path, "-c", config], cwd=root_dir)
    else:
        print(f"Warning: Solution file not found at {sln_path}. Trying 'test/' directory.")
        test_dir = os.path.join(root_dir, "test")
        if os.path.exists(test_dir):
            run_command(["dotnet", "test", test_dir, "-c", config], cwd=root_dir)
        else:
            print("Error: Could not find any .NET tests to run.")
            sys.exit(1)

    print("\n--- All Tests Finished ---")

if __name__ == "__main__":
    main()
