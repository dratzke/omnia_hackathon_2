# Define the download directory
$DownloadDir = "./release/latest"
if (-Not (Test-Path -Path $DownloadDir)) {
    New-Item -ItemType Directory -Path $DownloadDir | Out-Null
}

# Fetch the latest tag name
$LatestTag = gh release view --repo $Env:REPO --json tagName --jq .tagName
Write-Host "Latest release tag: $LatestTag"

# Download ZIP files from the latest release
gh release download $LatestTag --repo $Env:REPO --pattern "*.zip" --dir $DownloadDir

# Extract all downloaded ZIP files
Get-ChildItem -Path "$DownloadDir\*.zip" | ForEach-Object {
    Write-Host "Extracting $($_.FullName)..."
    Expand-Archive -Path $_.FullName -DestinationPath $DownloadDir -Force
}

Write-Host "Done."