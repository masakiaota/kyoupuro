#!/usr/bin/env python3
"""Manual play API adapter. All actions are explicitly supplied by the caller."""
from pathlib import Path
import play_global_20260928 as recorder
recorder.OUT = Path(__file__).resolve().parents[1] / 'play_saved_edits_20260928'
recorder.OUT.mkdir(exist_ok=True)
from play_global_20260928 import api, create, state, show, step, checkout, note, compare, sid
OUT = recorder.OUT
