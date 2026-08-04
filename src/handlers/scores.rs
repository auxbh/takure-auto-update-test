use anyhow::{Ok, Result};
use crate::{helpers, CONFIGURATION, TACHI_IMPORT_URL};
use crate::types::game::{PlayerData2Data, PlayData3Data};
use crate::types::tachi::{Difficulty, HitMeta, Import, ImportMeta, ImportScore, Judgements, Playtype, TachiLamp, Optional, Flare};
use log::{debug, info};
use either::Either;

pub fn process_scores(scores: Either<PlayerData2Data, PlayData3Data>) -> Result<()> {

    let card = if let Some(card) = helpers::get_current_card_id() {
        if !CONFIGURATION.cards.whitelist.is_empty()
            && !CONFIGURATION.cards.whitelist.contains(&card)
        {
            info!(
                "Card {} is not whitelisted, skipping score(s) submission",
                card
            );
            return Ok(());
        }

        card
    } else {
        info!("Card ID is not set, skipping score(s) submission");
        return Ok(());
    };

    let time_achieved = std::time::UNIX_EPOCH
        .elapsed()
        .map(|duration| duration.as_millis())
        .map_err(|err| anyhow::anyhow!("Could not get time from System {:#}", err))?;

    let (import_score, import_meta) = match &scores {
        Either::Left(scores) => {
            if scores.isgameover {
                debug!("Aborting: isgameover is true");
                return Ok(());
            }

            if scores.ref_id.starts_with("X000") {
                info!("Guest play, skipping score(s) submission");
                return Ok(());
            }

            let highest_stage = scores.note.iter()
                .filter(|n| n.stagenum != 0)
                .max_by_key(|n| n.stagenum)
                .cloned();

            match highest_stage {
                Some(highest_stage) => {
                    debug!("Selected note with highest stagenum != 0: {:#?}", highest_stage);

                    let import_score = ImportScore {
                        score: highest_stage.score,
                        lamp: TachiLamp::from(highest_stage.clearkind),
                        match_type: "inGameID".to_string(),
                        identifier: highest_stage.mcode.to_string(),
                        difficulty: Difficulty::from(highest_stage.notetype),
                        time_achieved,
                        judgements: Judgements {
                            marvelous: highest_stage.judge_marvelous,
                            perfect: highest_stage.judge_perfect,
                            great: highest_stage.judge_great,
                            good: highest_stage.judge_good,
                            miss: highest_stage.judge_miss,
                            ok: highest_stage.judge_ok,
                        },
                        optional: Optional {
                            hit_meta: HitMeta {
                                fast: highest_stage.fastcount,
                                slow: highest_stage.slowcount,
                                max_combo: highest_stage.maxcombo,
                                ex_score: highest_stage.ex_score,
                            },
                            flare: Flare::None,
                        },
                    };

                    let import_meta = ImportMeta {
                        game: "ddr".to_string(),
                        play_type: Playtype::from(highest_stage.playstyle),
                        service: "Takure".to_string(),
                    };

                    (import_score, import_meta)
                }
                None => {
                    debug!("No valid note with stagenum != 0 found, skipping");
                    return Ok(());
                }
            }
        }
        Either::Right(scores) => {
            if scores.ref_id.starts_with("X000") {
                info!("Guest play, skipping score(s) submission");
                return Ok(());
            }

            let result: crate::types::game::Result =
                serde_json::from_value(scores.result.clone())?;

            let import_score = ImportScore {
                score: result.score,
                lamp: TachiLamp::from(result.clearkind),
                match_type: "inGameID".to_string(),
                identifier: result.mcode.to_string(),
                difficulty: Difficulty::from(result.difficulty),
                time_achieved,
                judgements: Judgements {
                    marvelous: result.judge_marv,
                    perfect: result.judge_perf,
                    great: result.judge_great,
                    good: result.judge_good,
                    miss: result.judge_miss,
                    ok: result.judge_ok,
                },
                optional: Optional {
                    hit_meta: HitMeta {
                        fast: result.fastcount,
                        slow: result.slowcount,
                        max_combo: result.maxcombo,
                        ex_score: result.ex_score,
                    },
                    flare: if result.flare_force <= 0 {
                        Flare::None
                    } else {
                        Flare::from(result.flare_force as u8)
                    },
                },
            };

            let import_meta = ImportMeta {
                game: "ddr".to_string(),
                play_type: Playtype::from(result.style),
                service: "Takure".to_string(),
            };

            (import_score, import_meta)
        }
    };

    let import = Import {
        meta: import_meta,
        scores: vec![import_score],
    };

    if cfg!(debug_assertions) {
        debug!("Tachi API request data: {}", serde_json::to_string_pretty(&import).unwrap_or_else(|err| format!("Failed to serialize: {err}")));
    } else {
        helpers::call_tachi("POST", TACHI_IMPORT_URL.as_str(), Some(import))?;
        info!("Successfully imported score(s) for card {}", card);
    }

    Ok(())
}
