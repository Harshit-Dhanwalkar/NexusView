NexusView renders PDF content by utilizing the pdfium binaries. To enable PDF preview, you need to download the pdfium-binaries from [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases) and grab appropriate binary for your system (for linux I have used [pdfium-v8-linux-x64.tgz](https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7350/pdfium-linux-arm64.tgz) and then copy the extracted binaries from `pdfium/lib/libpdfium.so` to `/target/debug/` or for system-wide `/usr/lib/` directory.

```bash
tar -xvzf pdfium-linux-arm64.tgz
cp pdfium/lib/libpdfium.so /target/debug/
```
