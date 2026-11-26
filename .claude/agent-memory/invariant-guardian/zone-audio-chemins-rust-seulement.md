# La règle de zone audio ne vise que les chemins Rust

La règle « audio **et** UI dans le même diff → AVERTISSEMENT » ne s'applique qu'aux chemins
**Rust** de l'audio (`src-tauri/src/audio/**`, `src-tauri/src/coreaudio*`). Les fichiers
frontend du lecteur sont de l'UI malgré leur nom.

Faux positif rencontré lors de la relecture du correctif shuffle, le 2026-10-29.

Déplacé ici depuis `hean-invariants/SKILL.md` le 2026-11-25 : cette note ne sert qu'à l'agent
de relecture, et elle était lue par **chaque** session qui touchait au code.
