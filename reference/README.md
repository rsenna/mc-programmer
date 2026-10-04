# Reference documentation

This directory holds the **third-party documentation** that the
libraries derive from. Anything Roland-copyrighted is intentionally
**not** committed to the repository — see the policy below.

## What's here

| File | Source | Why it's safe to commit |
|---|---|---|
| `MC_Programming_Video.txt` | Transcript of [the MC sequencer programming video](https://youtu.be/_8IH1anKMNs) by the original test-script author | Third-party content released alongside the original test scripts under a non-restrictive license. Was in the repo from day one. |
| `ZC_Programming_Video.txt` | Transcript of the companion ZEN-Core programming video by the same author | Same as above. |

## What's *not* here (and how to get it locally)

The libraries' `zencore_data.py`, `ZenCoreData.swift`, and
`zencore_data.rs` parameter tables are auto-generated from Roland's
**Jupiter-X/Xm MIDI Implementation** PDF. Roland's PDFs are copyrighted, so
they are **not** committed to this repository. The auto-generated tables
themselves *are* committed, so the libraries work out of the box without you
ever needing to download anything from Roland.

You only need the source PDFs if you want to **regenerate or extend**
the parameter tables. In that case, download the relevant documents
from Roland's official support pages and save them locally in this
directory:

| File you'd save here | Where to download |
|---|---|
| `JUPITER-X_Xm_MIDI_imple_eng06_W.pdf` | [Roland Jupiter-X support page](https://www.roland.com/global/support/by_product/jupiter-x/owners_manuals/) → "MIDI Implementation" |
| `MC-707_Reference_eng01_W.pdf` | [Roland MC-707 support page](https://www.roland.com/global/support/by_product/mc-707/owners_manuals/) → "Reference Manual" |
| `MC-707_MIDIImpleChart_eng03_W.pdf` | Same page → "MIDI Implementation Chart" |
| `MC-707_eng02_W.pdf` | Same page → "Owner's Manual" |
| `MC-707_update_eng*.pdf` | Same page → "MC-707 Update" (firmware notes through Ver. 1.80) |
| `MC-101_Reference_eng01_W.pdf` | [Roland MC-101 support page](https://www.roland.com/global/support/by_product/mc-101/owners_manuals/) → "Reference Manual" |
| `MC-101_MIDIImpleChart_eng03_W.pdf` | Same page → "MIDI Implementation Chart" |
| `MC-101_eng02_W.pdf` | Same page → "Owner's Manual" |
| `MC-101_update_eng*.pdf` | Same page → "MC-101 Update" |

After downloading the Jupiter-X PDF, regenerate the parameter tables:

Regeneration requires Python 3, poppler, and `rustfmt` from a Rust toolchain.
```bash
# extract the relevant Jupiter-X PDF pages to text (needs poppler)
# macOS: brew install poppler  ·  Debian: apt install poppler-utils
pdftotext -raw -f 76 -l 82 \
    reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf \
    reference/tools_jp_zcore.txt

pdftotext -raw -f 85 -l 88 \
    reference/JUPITER-X_Xm_MIDI_imple_eng06_W.pdf - \
  | sed -n '/^* \[MFX\]/,/^* \[Reverb\]/p' \
  >> reference/tools_jp_zcore.txt

python tools/gen_zencore_params.py  # regenerates Python, Swift, and Rust tables
```

The `.gitignore` is configured to **block** `reference/*.pdf`,
`reference/firmware/`, and `reference/tools_jp_zcore.txt` so that any
PDFs you download for local development never accidentally get
committed.

## Why the auto-generated parameter tables are OK to commit

The committed `zencore_data.py`, `ZenCoreData.swift`, and `zencore_data.rs`
files contain **facts**: parameter offsets, sizes, ranges, and short technical
labels. Under US copyright law (*Feist Publications v. Rural Telephone
Service*, 1991), facts are not copyrightable; only the *creative
expression* used to present them is. Short functional terms like "Tone
Coarse Tune" and "TVF Cutoff Frequency" are technical terminology used
across the industry and aren't copyrightable as expression either.

This is the same reason third-party Roland editors (Ctrlr panels,
Sysex Librarian profiles, the various open-source librarians on
GitHub) routinely publish parameter tables for Roland gear without
licensing issues.

The actual *prose* in Roland's PDFs (the long explanations, the
introductory chapters, the diagrams) is copyrighted and is not
reproduced in this repo.

## Roland firmware

Roland firmware images (e.g. `MC707_UPA_up.bin`) are Roland's
intellectual property and are **not** committed to this repository.
If you have a local copy you've downloaded from Roland for personal
use, you can keep it under `reference/firmware/` and the `.gitignore`
will keep it out of commits.
