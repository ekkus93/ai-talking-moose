# Accessibility Contrast Audit

Date: 2026-08-24

Scope: shipped V1 frontend surfaces in `src/windows/` and `src/components/`

Standard: WCAG 2.2 AA contrast criteria (4.5:1 for normal text, 3:1 for large text and non-text UI indicators)

## Method

The audit uses the actual Tailwind CSS 3.4 palette values declared by the project plus the shipped retro surface colors (`#ece7de`, `#ded9cf`, `#dcd6cd`, `#fbf9f5`, `#fff9e6`, black, and white). Relative luminance and contrast ratios use the WCAG sRGB formula. The source sweep covered explicit text/background utility pairs, inherited retro surfaces, title-bar controls, state badges, transcript states, settings/onboarding cards, alerts, and the V1R-192 focus treatment.

Disabled controls are not treated as contrast failures because inactive UI components are exempt from WCAG 1.4.3/1.4.11. The partially transparent streaming transcript cards were evaluated after compositing their declared background alpha and group opacity over `#ece7de`; their gray-700 text remains above 5.4:1.

## Defects corrected by V1R-193

| Surface                  | Before                  | Before ratio | Corrected               | Corrected ratio | Result |
| ------------------------ | ----------------------- | -----------: | ----------------------- | --------------: | ------ |
| Listening state badge    | white / `green-600`     |       3.30:1 | white / `green-700`     |          5.02:1 | AA     |
| Thinking state badge     | white / `amber-600`     |       3.19:1 | white / `amber-700`     |          5.02:1 | AA     |
| Annoyed state badge      | white / `orange-600`    |       3.56:1 | white / `orange-700`    |          5.18:1 | AA     |
| Active Stop button       | white / `red-500`       |       3.76:1 | white / `red-600`       |          4.83:1 | AA     |
| Empty transcript copy    | `gray-500` / `#ece7de`  |       3.93:1 | `gray-600` / `#ece7de`  |          6.14:1 | AA     |
| Transcript prompt marker | `green-700` / `#dcd6cd` |       3.47:1 | `green-800` / `#dcd6cd` |          4.94:1 | AA     |

## Representative passing palette pairs

These pairs cover the remaining recurring V1 palette families after the source sweep.

| Use                                | Foreground / background   |   Ratio |
| ---------------------------------- | ------------------------- | ------: |
| Talking state                      | white / `blue-600`        |  5.17:1 |
| Interrupted state                  | white / `purple-600`      |  5.38:1 |
| Muted state                        | white / `gray-600`        |  7.56:1 |
| Sleeping state                     | white / `indigo-600`      |  6.29:1 |
| Error state                        | white / `red-600`         |  4.83:1 |
| Idle / primary buttons             | white / black             | 21.00:1 |
| Settings footer copy               | `gray-600` / `#ded9cf`    |  5.37:1 |
| Diagnostics helper copy            | `gray-500` / `#fbf9f5`    |  4.60:1 |
| Moose transcript label             | `amber-900` / `#fff9e6`   |  8.62:1 |
| User transcript label              | `blue-900` / white        | 10.36:1 |
| Standard secondary copy            | `gray-700` / white        | 10.31:1 |
| Success badge                      | `green-900` / `green-100` |  8.30:1 |
| Warning badge                      | `amber-900` / `amber-100` |  8.15:1 |
| Error alert                        | `red-800` / `red-50`      |  7.60:1 |
| Success alert                      | `green-800` / `green-50`  |  6.81:1 |
| Neutral panel                      | `gray-800` / `gray-50`    | 14.05:1 |
| Transcript title icon              | `green-400` / black       | 12.05:1 |
| Transcript title secondary control | `gray-400` / black        |  8.27:1 |
| Transcript destructive hover       | `red-400` / black         |  7.59:1 |
| V1R-192 focus indicator on black   | white / black             | 21.00:1 |

The corrected state badge minimum is 4.83:1 and the minimum normal-text contrast found in the audited shipped palette is the diagnostics `gray-500` on `#fbf9f5` pair at 4.60:1. No remaining active normal-text pair in the audited V1 surfaces is below 4.5:1.

## Regression coverage

Frontend regressions assert that the rendered production `MooseWindow` uses the corrected AA state/Stop classes and that the rendered production `TranscriptDrawer` uses the corrected readable empty-state and prompt-marker classes. This makes the specific failures found by this audit visible to the existing Vitest gate rather than leaving the audit as documentation only.
