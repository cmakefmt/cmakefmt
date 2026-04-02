# `tests/snapshots/`

This directory stores checked-in snapshot baselines.

## Purpose

Each snapshot records the expected formatted output for a named test case or
real-world fixture. CI needs these files, so they are version-controlled.

## What To Commit

- commit the `.snap` files that represent accepted expected output
- do not commit `*.pending-snap`

## When They Change

Update snapshots when:

- formatter behavior changed intentionally
- a real-world fixture changed intentionally
- a test was renamed and the snapshot key moved

If a snapshot diff is surprising, treat that as a possible formatter regression
until proven otherwise.
