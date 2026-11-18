---
type: decision
status: verified
verifiedAgainst: working tree on 8a1e79e (screen sweep), 2026-11-18
sourceRefs:
  - sight/PERF.md#A fixed 3° step lost the 27° telephoto Daybreak photo (39 km off); a 1° step at 27° keeps it.
  - sight/PERF.md#so `yaw_span` is gone and the screen's best window is now exact over whole yaws.
---

# Decision: screen yaw step (superseded: every yaw)

**Now:** the screen scores every whole yaw ([[locator/screen/screen-templates]]). The stepped scan below was a speed measure; once the cross terms of all 360 windows came from one tiled loop, sweeping every yaw was faster than stepping (the screen benchmark went from 9.03 ms stepped to 7.70 ms for all yaws), so there is no step left to choose.

**History, still the lesson:** the screen used to try every ⌊fov/25⌋ degrees (at least 1), then the degrees between around the best. A fixed 3° step for all lenses had been rejected: the fit's sharpness in yaw scales with the lens. A 3° miss is a tenth of a 27° frame, enough to wreck the correlation, but only a thirtieth of a 100° frame. The 27° Daybreak telephoto lost its place (39 km off) at 3°.

**What would change it:** a reason to subsample yaws again (for example a much more expensive window). Then the step must scale with the lens, and Daybreak is the photo to check.
