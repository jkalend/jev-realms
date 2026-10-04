//! Save-file persistence shared by the terminal and windowed views.

use crate::model::{Game, Modal};
use std::fs;
use std::path::PathBuf;

pub fn save_path(seed: u64) -> PathBuf {
    PathBuf::from(format!("saves/journey-{seed}.json"))
}

pub fn save_game(game: &Game) -> Result<(), String> {
    fs::create_dir_all("saves").map_err(|e| format!("save directory: {e}"))?;
    let data = serde_json::to_string(game).map_err(|e| format!("save serialize: {e}"))?;
    let path = save_path(game.seed);
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, data).map_err(|e| format!("save write: {e}"))?;
    let _ = fs::remove_file(&path);
    fs::rename(&tmp_path, &path).map_err(|e| format!("save rename: {e}"))
}

pub fn try_load(game: &mut Game) {
    let path = save_path(game.seed);
    if !path.exists() {
        game.log("No saved journey for this seed. Press Enter to begin anew.");
        return;
    }
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            game.log(format!("Failed to read saved journey: {e}. Press Enter to begin anew."));
            return;
        }
    };
    let mut restored = match serde_json::from_str::<Game>(&data) {
        Ok(g) => g,
        Err(e) => {
            game.log(format!("Saved journey corrupted or incompatible: {e}. Press Enter to begin anew."));
            return;
        }
    };
    // E7 (§6.5): the keep's lord has his name back (the §4.1 canon). Old saves
    // store NPC names literally, so the rename lands as a one-time load note.
    let mut renamed = false;
    for npc in &mut restored.npcs {
        if npc.archetype == crate::model::Archetype::Lich
            && (npc.name == "The Lich" || npc.name == "Lich")
        {
            npc.name = "Vael, the Last Castellan".into();
            renamed = true;
        }
    }
    if renamed {
        restored.log("The keep's lord has a name again: Vael, the Last Castellan.");
    }
    // Async decision state cannot outlive the session; everything else persists.
    restored.outbox.clear();
    restored.social_pending = None;
    restored.ai.pending = 0;
    for npc in &mut restored.npcs {
        npc.decision_pending = false;
    }
    restored.ai.provider = game.ai.provider.clone();
    restored.ai.switched = game.ai.switched;
    // move_ready_ms is an absolute wall-clock gate; the session clock restarts at
    // zero on every launch. Without the rebase, movement would stay locked until
    // the fresh clock overtook the saved value.
    restored.move_ready_ms = 0;
    restored.modal = Modal::None;
    restored.quit = false;
    restored.log("Journey restored. The road remembered your place in it.");
    *game = restored;
}
