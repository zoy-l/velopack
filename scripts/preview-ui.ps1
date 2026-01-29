# Velopack UI Preview Script

Write-Host "--- Velopack Slint UI Preview ---" -ForegroundColor Cyan
Write-Host "1. Preview Setup UI (Installer)"
Write-Host "2. Preview Splash UI (Updater/Splash)"
Write-Host "3. Preview Overwrite UI (Repair/Update)"
Write-Host "4. Build All Bins (Verify compilation)"
Write-Host "Q. Quit"

$choice = Read-Host "Select an option"

if ($choice -eq "1") {
    Write-Host "Starting Setup UI Preview..." -ForegroundColor Green
    cargo test -p velopack_bins windows::splash::tests::test_show_setup -- --ignored --nocapture
} elseif ($choice -eq "2") {
    Write-Host "Starting Splash UI Preview..." -ForegroundColor Green
    cargo test -p velopack_bins windows::splash::tests::test_show_splash -- --ignored --nocapture
} elseif ($choice -eq "3") {
    Write-Host "Starting Overwrite UI Preview..." -ForegroundColor Green
    cargo test -p velopack_bins windows::splash::tests::test_show_overwrite -- --ignored --nocapture
} elseif ($choice -eq "4") {
    Write-Host "Building bins..." -ForegroundColor Yellow
    cargo build -p velopack_bins
} elseif ($choice -eq "q" -or $choice -eq "Q") {
    exit
} else {
    Write-Host "Invalid option." -ForegroundColor Red
}
