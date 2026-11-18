---
type: moc
status: verified
verifiedAgainst: 3b6ccf9, 2026-11-18
---

# Scoring (the full search)

At one place: for every lens (48) × requested yaw (360, or ±8° around the screen's yaw), guess the tilt, measure the fit, keep the lowest loss.

- [[measure]]: one camera at one yaw: trimmed residuals, pitch, RMSE, relief, correlation.
- [[fit-and-tilt-guess]]: picking the tilted camera from the level one, with one retry.
- [[loss]]: how the terms add up, and the story behind each.
- [[best-yaw-and-ties]]: the yaw and lens loops, the tie rule, ordering.
