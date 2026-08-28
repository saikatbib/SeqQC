SeqQC Studio - Engineering Preview
==================================

SeqQC Studio is a local-first native desktop application. It does not need an
Internet connection to start or inspect local sequencing inputs.

Windows
-------
Extract the entire ZIP archive, then double-click "SeqQC Studio.exe".
For a normal installation with Start-menu, Installed Apps, and uninstall
entries, download the Windows Setup executable from the GitHub Release instead.

macOS (Apple silicon)
---------------------
Extract the entire ZIP archive and move "SeqQC Studio.app" to Applications.
This preview is not signed or notarized. On first launch, Control-click the app,
choose Open, and confirm that you want to open it. Do not bypass a warning if the
package checksum does not match the SHA256SUMS.txt file on the GitHub Release.

Linux (x86-64)
----------------
Extract the archive, open the extracted directory, and run:

    ./seqqc-studio

The preview is built on Ubuntu 24.04 and requires a graphical Linux desktop.

Important limitations
---------------------
This is an early engineering preview. It demonstrates the offline native shell
and local sequencing-file discovery. It is not yet validated for clinical or
diagnostic use, and it does not yet run the complete SeqQC analysis pipeline.

Project: https://github.com/saikatbib/SeqQC
License: Apache-2.0
