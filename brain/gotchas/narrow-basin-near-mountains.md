---
type: gotcha
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
sourceRefs:
  - CLAUDE.md#Near mountains the good fit is only ~125 m wide, so the 2 km field alone misses it
  - sight/crates/sight-core/src/field.rs#Whether the ground within `radius_m` of the place spans at least `relief_m`
---

# Narrow basin near mountains

**Symptom:** a photo taken in or near a canyon mouth or foothills lands tens of km away (Little Cottonwood: 51 km off with the 2 km field alone; 14.8 km with the first screen).

**Cause:** near steep ground the skyline changes by degrees within a few hundred meters, because the nearest ridge is close. The region where the photo fits is ~125 m wide. A 2 km lattice has no place inside it, so the nearest lattice place fits badly, and some far place with a vaguely similar skyline wins.

**Fix:** the rugged 500 m field, built only where ground within 3 km spans ≥ 300 m (29,669 places), plus refinement at 250 m and 100 m ([[locator/search/refinement]]). Always search both fields. Little Cottonwood now lands 0.12 km off, truth rank 1.

**Still possible:** a photo whose good region is narrower than ~500 m and whose 500 m neighbours fit poorly enough to be screened out. `sight around` on the true spot tells whether the ground there fits at all.
