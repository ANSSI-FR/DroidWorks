use crate::errors::Error;
use crate::read_state;
use crate::state::DwState;
use std::fs::File;
use std::io::Write;
use tauri::{command, State};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DexStringsRequest {
    start: usize,
    nb: usize,
    filter: String,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DexStringsResponse {
    nb_total: usize,
    data: Vec<String>,
}

#[command]
pub async fn extract_dex_strings(filename: String, state: State<'_, DwState>) -> Result<(), Error> {
    read_state!(state.package => |package| {
        let mut file = File::create(filename)?;
        let strings = package.dexs_strings()?;
        for (pos, s) in strings.iter().enumerate() {
            writeln!(file, "[{}] = {:?}", pos, s)?;
        }
        Ok(())
    })
}

#[command]
pub async fn dex_strings(
    req: DexStringsRequest,
    state: State<'_, DwState>,
) -> Result<DexStringsResponse, Error> {
    if state
        .dexs_strings
        .read()
        .map_err(|_| Error::Internal(300))?
        .is_none()
        || *state
            .dexs_strings_filter
            .read()
            .map_err(|_| Error::Internal(301))?
            != req.filter
    {
        read_state!(state.package => |package| {
            let strings = package.dexs_strings()?;
            let mut result = Vec::with_capacity(strings.len());
            for s in strings {
                if s.contains(&req.filter) {
                    let mut es = String::new();
                    html_escape::encode_text_to_string(s, &mut es);
                    result.push(es);
                }
            }
            *state.dexs_strings.write().map_err(|_| Error::Internal(302))? = Some(result);
            *state.dexs_strings_filter.write().map_err(|_| Error::Internal(303))? = req.filter;
            Ok(())
        })
    } else {
        Ok(())
    }?;

    read_state!(state.dexs_strings => |strings| {
        Ok(DexStringsResponse {
            nb_total: strings.len(),
            data: strings
                .iter()
                .skip(req.start)
                .take(req.nb)
                .cloned()
                .collect(),
        })
    })
}
