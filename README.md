# rOSEK

Ein AUTOSAR-OS-konformer Realtime-Kernel (SWS_OS) in Rust für die
BeagleBone Black (TI AM335x, ARM Cortex-A8) — von Beginn an so gebaut,
dass er sich auf andere SoCs und Board-Derivate portieren lässt.

Clean-Room-Implementierung nach der offenen AUTOSAR-OS-Spezifikation —
kein Fork oder Port von ERIKA Enterprise oder anderem bestehenden Code.
Der Name spielt auf OSEK (die Standard-Basis von AUTOSAR OS) an,
kombiniert mit Rust — kein Code- oder Lizenzbezug zu ERIKA Enterprise.

## Portierbarkeit

Drei Schichten, analog zur MCAL/ECU-Abstraction-Trennung in AUTOSAR:

- **rosek-core** — architektur- und boardneutral: Scheduler, OS-Application-Modell,
  Memory-Protection-Logik, Static-Config-Verarbeitung. Kennt nur HAL-Traits, keine Register.
- **rosek-hal** — die Trait-Schnittstellen (`Timer`, `InterruptController`, `Mmu`, `Uart`, `Gpio`),
  die rosek-core braucht.
- **rosek-port-<soc-familie>** (z. B. `rosek-port-am335x`) — HAL-Implementierung für eine SoC-Familie.
- **rosek-board-<derivat>** (z. B. `rosek-board-beaglebone-black`) — nur das Board-Spezifische:
  Pin-Muxing, Taktkonfiguration, Speicherkarte.

Ein neues Target = ein neuer `rosek-port-*` (neuer SoC) oder `rosek-board-*`
(neues Board, bekannter SoC) — nie eine Änderung an `rosek-core`.

## Status

Phase 0 (Boot-Fundament) in Arbeit. Siehe [PLAN.md](./PLAN.md) für
Architektur, Meilensteine und offene Entscheidungen, sowie
[docs/req42/](./docs/req42/) und [docs/arc42/](./docs/arc42/) für
Requirements und Architekturdokumentation.

## Zielhardware (erstes Target)

- TI AM335x, ARM Cortex-A8 (ARMv7-A), Single-Core
- UART0-Konsole, User-LEDs über GPIO1
- Boot über bestehendes u-boot (Raw-Binary, kein eigener MLO/X-Loader)

## Lizenz

Noch offen (siehe PLAN.md, Abschnitt "Offene Punkte").
