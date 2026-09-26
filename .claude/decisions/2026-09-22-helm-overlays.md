# Helm's dock takes 380px from every screen.

**Decided 2026-09-22.**

Three screens hit it independently and each worked around it locally.

**Chosen:** Helm overlays the content and starts shut, so the dock asks for no width at all.

**Cost he took:** The overlay can cover an actionable control; shut by default and one press to close are the mitigations.

**Where it landed:** #1583
