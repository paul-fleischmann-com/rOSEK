# rustika

Ein AUTOSAR-OS-konformer Realtime-Kernel (SWS_OS) in Rust für die
BeagleBone Black (TI AM335x, ARM Cortex-A8).

Clean-Room-Implementierung nach der offenen AUTOSAR-OS-Spezifikation —
kein Fork oder Port von ERIKA Enterprise oder anderem bestehenden Code.
Der Name spielt auf ERIKA als bekannte OSEK/AUTOSAR-RTOS-Referenz an,
ist aber ein eigenständiges Projekt ohne Code- oder Lizenzbezug dazu.

## Status

Planungsphase. Siehe [PLAN.md](./PLAN.md) für Architektur, Meilensteine
und offene Entscheidungen.

## Zielhardware

- TI AM335x, ARM Cortex-A8 (ARMv7-A), Single-Core
- UART0-Konsole, User-LEDs über GPIO1
- Boot über bestehendes u-boot (Raw-Binary, kein eigener MLO/X-Loader)

## Lizenz

Noch offen (siehe PLAN.md, Abschnitt "Offene Punkte").
