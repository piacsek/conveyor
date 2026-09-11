# Phase 12 — two things the cards were getting wrong

## 1. A cancelled run blamed the jobs its cancellation killed

GitHub marks the jobs a cancellation stops as **failed**, so a cancelled build card carried a
red `✗ Nx / Status · Check Nx workflow status` line for work that never failed on its own — the
loudest thing on the card, and wrong. `build_jobs` and the queue's `failed_job` now drop the
job line when the run is `Cancelled` and fall back to the sha. The details pane still lists
every job: there the conclusions are stated as facts, not as a headline.

## 2. The title shared the first line with the number and the age

`✓ #7108 [AIE-65] Remove commit and push… 3h` — the title was the part that got clipped, and it
is the one thing on the card you cannot reconstruct from the others. A build card is four lines
now: the number with the age, then the title on a line of its own at full width, then the meta
line, then the jobs line.

Row `n` starts at screen line `1 + 4n`; card tests index accordingly, and a test that wants a
third card needs a terminal taller than the 12-row default.
