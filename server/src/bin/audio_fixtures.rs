//! Optional development TTS synthesis. This binary has no call-control executor.
use anyhow::{Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    schema_version: u32,
    split: String,
    data_origin: String,
    authored_before_generation: bool,
    model_id: String,
    output_format: String,
    voice_slots: Vec<String>,
    scenarios: Vec<Scenario>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    group_id: String,
    segment_index: usize,
    prior_segments: Vec<String>,
    split: String,
    text: String,
    profile: String,
    expected: Expected,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    classification: String,
    acceptable_actions: Vec<String>,
    must_not_bridge_without_current_consent: bool,
    must_not_execute_unenrolled_call: bool,
    assessment_required: bool,
    rationale: String,
}
#[derive(Clone, Debug, Serialize)]
struct Clip {
    id: String,
    group_id: String,
    scenario_id: String,
    segment_index: usize,
    voice_slot: String,
    voice_id: Option<String>,
    text: String,
    text_sha256: String,
    render_sha256: Option<String>,
    split: String,
}
const API: &str = "https://api.elevenlabs.io";
const MAX_CLIPS: usize = 36;
const MAX_AUDIO_BYTES: usize = 2 * 1024 * 1024;
fn token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn validate(data: &Dataset) -> Result<()> {
    ensure!(
        data.schema_version == 1
            && data.split == "development"
            && data.data_origin == "synthetic"
            && data.authored_before_generation,
        "Only authored synthetic development data is accepted"
    );
    ensure!(
        data.model_id == "eleven_multilingual_v2" && data.output_format == "mp3_44100_128",
        "Unexpected audio model/format"
    );
    ensure!(
        (1..=12).contains(&data.scenarios.len()) && (1..=3).contains(&data.voice_slots.len()),
        "Plan must contain 1..12 scenarios and 1..3 voice slots"
    );
    ensure!(
        data.voice_slots.iter().all(|s| token(s))
            && data.voice_slots.iter().collect::<HashSet<_>>().len() == data.voice_slots.len(),
        "Voice slots must be unique safe identifiers"
    );
    let mut seen = HashMap::<&str, &Scenario>::new();
    for case in &data.scenarios {
        ensure!(
            token(&case.id) && token(&case.group_id) && !seen.contains_key(case.id.as_str()),
            "Invalid or duplicate scenario identifier"
        );
        ensure!(
            case.split == data.split
                && ["a_only", "fictional_enrolled"].contains(&case.profile.as_str()),
            "Split or capability profile mismatch"
        );
        ensure!(
            !case.text.trim().is_empty()
                && case.text.chars().count() <= 500
                && !case.text.chars().any(char::is_control),
            "Invalid bounded plain-text utterance"
        );
        ensure!(
            case.segment_index == case.prior_segments.len(),
            "Segment context is incomplete"
        );
        for (index, prior) in case.prior_segments.iter().enumerate() {
            ensure!(
                seen.get(prior.as_str()).is_some_and(|segment| {
                    segment.group_id == case.group_id
                        && segment.segment_index == index
                        && segment.prior_segments == case.prior_segments[..index]
                }),
                "Prior segments must exist in order within the same development group"
            );
        }
        ensure!(
            ["benign", "risky", "clarify"].contains(&case.expected.classification.as_str())
                && !case.expected.acceptable_actions.is_empty()
                && case.expected.acceptable_actions.iter().all(|a| [
                    "ask_caller",
                    "take_message",
                    "offer_handoff",
                    "lookup_trusted_contact",
                    "start_trusted_callback"
                ]
                .contains(&a.as_str())),
            "Invalid independently authored rubric"
        );
        ensure!(
            case.expected.assessment_required
                && case.expected.must_not_bridge_without_current_consent
                && case.expected.must_not_execute_unenrolled_call
                && !case.expected.rationale.trim().is_empty(),
            "Missing policy expectation"
        );
        seen.insert(&case.id, case);
    }
    Ok(())
}
fn plan(data: &Dataset, voices: &[String]) -> Result<Vec<Clip>> {
    validate(data)?;
    ensure!(
        voices.is_empty()
            || (voices.len() == data.voice_slots.len()
                && voices.iter().all(|v| v.len() >= 8
                    && v.len() <= 64
                    && v.bytes().all(|b| b.is_ascii_alphanumeric()))
                && voices.iter().collect::<HashSet<_>>().len() == voices.len()),
        "Choose one distinct provider voice ID per selected slot, or omit them for dry-run"
    );
    let mut clips = Vec::new();
    for case in &data.scenarios {
        for (index, slot) in data.voice_slots.iter().enumerate() {
            let mut clip = Clip {
                id: format!("{}_{}", case.id, slot),
                group_id: case.group_id.clone(),
                scenario_id: case.id.clone(),
                segment_index: case.segment_index,
                voice_slot: slot.clone(),
                voice_id: voices.get(index).cloned(),
                text: case.text.clone(),
                text_sha256: format!("{:x}", Sha256::digest(case.text.as_bytes())),
                render_sha256: None,
                split: data.split.clone(),
            };
            clip.render_sha256 = clip
                .voice_id
                .as_deref()
                .map(|voice| render_hash(&payload(data, &clip), voice, &data.output_format));
            clips.push(clip);
        }
    }
    ensure!(clips.len() <= MAX_CLIPS, "Audio request budget exceeded");
    Ok(clips)
}
fn generation_key(generate: bool, key: Option<String>) -> Result<Option<String>> {
    if !generate {
        return Ok(None);
    }
    let key = key
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| anyhow!("ELEVENLABS_API_KEY missing; zero provider requests submitted"))?;
    Ok(Some(key))
}
fn voice_settings() -> Value {
    json!({"stability":0.5,"similarity_boost":0.75,"style":0.0,"use_speaker_boost":true})
}
fn payload(data: &Dataset, clip: &Clip) -> Value {
    json!({"text":clip.text,"model_id":data.model_id,"voice_settings":voice_settings()})
}
// Compact UTF-8 JSON, recursively sorted object keys. Labels and clip IDs are
// deliberately excluded: a rubric edit must not change an audio render identity.
fn stable_json(value: &Value) -> Value {
    match value {
        Value::Object(fields) => {
            let sorted = fields.iter().collect::<std::collections::BTreeMap<_, _>>();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, value)| (key.clone(), stable_json(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.iter().map(stable_json).collect()),
        value => value.clone(),
    }
}
fn render_hash(request: &Value, voice: &str, output_format: &str) -> String {
    let identity = stable_json(
        &json!({"text":request["text"],"voice_id":voice,"model_id":request["model_id"],"output_format":output_format,"voice_settings":request["voice_settings"]}),
    );
    format!("{:x}", Sha256::digest(identity.to_string().as_bytes()))
}
fn atomic_create(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.partial", uuid::Uuid::new_v4()));
    let outcome = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        // Atomic no-clobber publication: hard_link fails if the target exists.
        std::fs::hard_link(&temporary, path)?;
        Ok::<_, anyhow::Error>(())
    })();
    let _ = std::fs::remove_file(&temporary);
    outcome.map_err(|_| anyhow!("Cannot publish output without overwriting existing work"))
}
#[derive(Debug)]
struct Options {
    generate: bool,
    voices: Vec<String>,
    limit: usize,
    dataset: String,
    scenarios: Vec<String>,
    voice_slot: Option<String>,
}
fn options(args: &[String]) -> Result<Options> {
    let mut result = Options {
        generate: false,
        voices: vec![],
        limit: MAX_CLIPS,
        dataset: "fixtures/audio-development.json".into(),
        scenarios: vec![],
        voice_slot: None,
    };
    let mut mode = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" | "--generate" => {
                ensure!(mode.is_none(), "Choose one execution mode");
                result.generate = args[i] == "--generate";
                mode = Some(result.generate);
            }
            "--voice-ids" => {
                i += 1;
                result.voices = args
                    .get(i)
                    .ok_or_else(|| anyhow!("--voice-ids requires comma-separated IDs"))?
                    .split(',')
                    .map(str::to_owned)
                    .collect();
            }
            "--dataset" => {
                i += 1;
                result.dataset = args
                    .get(i)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| anyhow!("--dataset requires a local JSON path"))?
                    .clone();
            }
            "--scenario" => {
                i += 1;
                let id = args
                    .get(i)
                    .filter(|value| token(value))
                    .ok_or_else(|| anyhow!("--scenario requires a safe scenario ID"))?;
                ensure!(
                    !result.scenarios.contains(id),
                    "Duplicate scenario selection"
                );
                result.scenarios.push(id.clone());
            }
            "--voice-slot" => {
                i += 1;
                ensure!(result.voice_slot.is_none(), "Choose at most one voice slot");
                result.voice_slot = Some(
                    args.get(i)
                        .filter(|value| token(value))
                        .ok_or_else(|| anyhow!("--voice-slot requires a safe slot name"))?
                        .clone(),
                );
            }
            "--limit" => {
                i += 1;
                result.limit = args
                    .get(i)
                    .ok_or_else(|| anyhow!("--limit requires 1..36"))?
                    .parse()
                    .map_err(|_| anyhow!("--limit requires 1..36"))?;
                ensure!(
                    (1..=MAX_CLIPS).contains(&result.limit),
                    "--limit requires 1..36"
                );
            }
            _ => bail!("Unknown option; use --help"),
        }
        i += 1;
    }
    Ok(result)
}
fn selected_plan(data: &Dataset, options: &Options) -> Result<Vec<Clip>> {
    // Validate all authored paths/rubrics before selecting render-only subsets.
    // An unselected opening stays in the source; selection never reauthors it.
    validate(data)?;
    let mut selected = data.clone();
    if let Some(slot) = &options.voice_slot {
        ensure!(data.voice_slots.contains(slot), "Unknown voice slot");
        selected.voice_slots = vec![slot.clone()];
    }
    for id in &options.scenarios {
        ensure!(
            data.scenarios.iter().any(|scenario| &scenario.id == id),
            "Unknown scenario selection"
        );
    }
    let mut clips = plan(&selected, &options.voices)?;
    if !options.scenarios.is_empty() {
        clips.retain(|clip| options.scenarios.contains(&clip.scenario_id));
    }
    ensure!(
        !clips.is_empty() && clips.len() <= MAX_CLIPS,
        "Invalid selected request count"
    );
    Ok(clips)
}
async fn metadata(client: &reqwest::Client, key: &str, path: &str) -> Result<Value> {
    let mut response = client
        .get(format!("{API}{path}"))
        .header("xi-api-key", key)
        .send()
        .await
        .map_err(|_| anyhow!("ElevenLabs metadata transport failed"))?;
    ensure!(
        response.status().is_success(),
        "ElevenLabs metadata HTTP {}",
        response.status().as_u16()
    );
    ensure!(
        response.content_length().unwrap_or(0) <= 1024 * 1024,
        "Metadata exceeded bound"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow!("ElevenLabs metadata body failed"))?
    {
        ensure!(
            bytes.len() + chunk.len() <= 1024 * 1024,
            "Metadata exceeded bound"
        );
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| anyhow!("ElevenLabs returned invalid metadata"))
}
async fn synthesize(
    client: &reqwest::Client,
    key: &str,
    data: &Dataset,
    clip: &Clip,
) -> Result<Vec<u8>> {
    let voice = clip
        .voice_id
        .as_deref()
        .ok_or_else(|| anyhow!("Voice mapping is missing"))?;
    let mut response = client
        .post(format!("{API}/v1/text-to-speech/{voice}"))
        .query(&[("output_format", data.output_format.as_str())])
        .header("xi-api-key", key)
        .header("Accept", "audio/mpeg")
        .json(&payload(data, clip))
        .send()
        .await
        .map_err(|_| anyhow!("TTS transport/timeout failure; billing outcome unknown, no retry"))?;
    ensure!(
        response.status().is_success(),
        "TTS HTTP {}; no retry",
        response.status().as_u16()
    );
    ensure!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("audio/")),
        "TTS returned a non-audio response"
    );
    ensure!(
        response.content_length().unwrap_or(0) <= MAX_AUDIO_BYTES as u64,
        "Audio exceeded size bound"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow!("Audio transfer incomplete; no retry"))?
    {
        ensure!(
            bytes.len() + chunk.len() <= MAX_AUDIO_BYTES,
            "Audio exceeded size bound"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(
        bytes.starts_with(b"ID3")
            || (bytes.len() > 1 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0),
        "Response is not identifiable MP3 audio"
    );
    Ok(bytes)
}
async fn generate(
    data: &Dataset,
    clips: &[Clip],
    key: &str,
    limit: usize,
    dir: &Path,
    results: &mut Vec<Value>,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let models = metadata(&client, key, "/v1/models").await?;
    ensure!(
        models.as_array().is_some_and(|rows| rows
            .iter()
            .any(|m| m["model_id"] == data.model_id && m["can_do_text_to_speech"] == true)),
        "Configured model is unavailable for TTS"
    );
    for voice in clips
        .iter()
        .filter_map(|c| c.voice_id.as_ref())
        .collect::<HashSet<_>>()
    {
        let info = metadata(&client, key, &format!("/v1/voices/{voice}")).await?;
        ensure!(
            info["voice_id"] == voice.as_str(),
            "Voice metadata mismatch"
        );
    }
    for clip in clips.iter().take(limit) {
        atomic_create(
            &dir.join(format!("{}.attempt.json", clip.id)),
            &serde_json::to_vec_pretty(
                &json!({"clip_id":clip.id,"text_sha256":clip.text_sha256,"render_sha256":clip.render_sha256,"status":"request_reserved","started_at":chrono::Utc::now().to_rfc3339(),"retry":false}),
            )?,
        )?;
        let start = Instant::now();
        let bytes = match synthesize(&client, key, data, clip).await {
            Ok(bytes) => bytes,
            Err(error) => {
                results.push(json!({"clip_id":clip.id,"text_sha256":clip.text_sha256,"render_sha256":clip.render_sha256,"status":"failed_or_unknown","error":error.to_string(),"elapsed_ms":start.elapsed().as_millis()}));
                return Err(error);
            }
        };
        let filename = format!("{}.mp3", clip.id);
        atomic_create(&dir.join(&filename), &bytes)?;
        let record = json!({"clip_id":clip.id,"text_sha256":clip.text_sha256,"render_sha256":clip.render_sha256,"status":"synthesized","file":filename,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"elapsed_ms":start.elapsed().as_millis(),"listened":false,"phone_tested":false});
        atomic_create(
            &dir.join(format!("{}.receipt.json", clip.id)),
            &serde_json::to_vec_pretty(&record)?,
        )?;
        results.push(record);
        println!("{}", json!({"clip_id":clip.id,"status":"synthesized"}));
    }
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--help"] {
        println!(
            "Optional FirstRing development audio fixtures. Default/--dry-run writes a 36-clip plan with zero network requests. --dataset PATH selects an authored development JSON file; repeat --scenario ID to render selected segments; --voice-slot NAME selects one slot. --voice-ids ID1[,ID2,ID3] must map one distinct ID per selected slot. --generate [--limit 1..36] explicitly authorizes up to 36 sequential billable TTS requests plus one model and up to three voice metadata reads; no retries, 20s/request and 180s/run. Requires local ELEVENLABS_API_KEY. Output: ignored .tools/audio-fixtures/<run>/. Never dials, executes model tools, changes live configuration, or grades held-out data."
        );
        return Ok(());
    }
    let options = options(&args)?;
    dotenvy::dotenv().ok();
    let key = generation_key(options.generate, std::env::var("ELEVENLABS_API_KEY").ok())?;
    let raw = std::fs::read(&options.dataset)
        .map_err(|_| anyhow!("Cannot read the selected local audio development dataset"))?;
    let data: Dataset =
        serde_json::from_slice(&raw).map_err(|_| anyhow!("Audio fixture schema is invalid"))?;
    let clips = selected_plan(&data, &options)?;
    ensure!(
        !options.generate || clips.iter().all(|c| c.voice_id.is_some()),
        "--generate requires one --voice-ids mapping per selected slot; zero provider requests submitted"
    );
    let dir = std::path::PathBuf::from(".tools/audio-fixtures").join(format!(
        "{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir)?;
    let total_characters: usize = clips.iter().map(|c| c.text.chars().count()).sum();
    let selected_characters: usize = clips
        .iter()
        .take(options.limit)
        .map(|c| c.text.chars().count())
        .sum();
    let total_words: usize = clips
        .iter()
        .map(|c| c.text.split_whitespace().count())
        .sum();
    let manifest = json!({"total_characters":total_characters,"selected_characters":selected_characters,"estimated_duration_seconds":{"min":total_words as f64/3.0,"max":total_words as f64/2.0,"method":"120-180 spoken words/minute estimate; not measured audio"},"schema_version":1,"split":"development","data_origin":"synthetic","operation":"fixture_synthesis","phone_tested":false,"telephony_actions":0,"provider":"elevenlabs","model_id":data.model_id,"output_format":data.output_format,"voice_settings":voice_settings(),"render_hash_scheme":"sha256:compact-utf8-json-recursively-sorted-keys:v1","source_sha256":format!("{:x}",Sha256::digest(&raw)),"planned_clips":clips.len(),"selected_request_limit":options.limit,"mode":if options.generate{"generate"}else{"dry_run"},"voice_mapping_pending":options.voices.is_empty(),"clips":clips,"rubric_file":options.dataset,"selected_scenario_ids":options.scenarios,"selected_voice_slot":options.voice_slot,"expected_labels_sent_to_tts":false});
    atomic_create(
        &dir.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    let mut results = Vec::new();
    let outcome = if let Some(key) = key {
        match tokio::time::timeout(
            Duration::from_secs(180),
            generate(&data, &clips, &key, options.limit, &dir, &mut results),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(anyhow!(
                "Run deadline reached; last request may be billed, no retry"
            )),
        }
    } else {
        Ok(())
    };
    let summary = json!({"status":if outcome.is_err(){"failed_or_incomplete"}else if options.generate{"synthesis_completed"}else{"dry_run"},"planned_clips":clips.len(),"synthesized_clips":results.iter().filter(|v|v["status"]=="synthesized").count(),"error":outcome.as_ref().err().map(ToString::to_string),"results":results,"phone_tested":false,"telephony_actions":0});
    atomic_create(
        &dir.join("results.json"),
        &serde_json::to_vec_pretty(&summary)?,
    )?;
    println!(
        "{}",
        json!({"status":summary["status"],"planned_clips":clips.len(),"total_characters":total_characters,"estimated_seconds_min":total_words as f64/3.0,"estimated_seconds_max":total_words as f64/2.0,"synthesized_clips":summary["synthesized_clips"],"manifest":dir.join("manifest.json"),"telephony_actions":0})
    );
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    fn data() -> Dataset {
        serde_json::from_str(include_str!("../../../fixtures/audio-development.json")).unwrap()
    }
    fn family_data() -> Dataset {
        serde_json::from_str(include_str!(
            "../../../fixtures/audio-family-development.json"
        ))
        .unwrap()
    }
    #[test]
    fn small_family_dataset_renders_one_shared_opening_and_two_authored_branch_clips() {
        let data = family_data();
        let clips = plan(&data, &["JBFqnCBsd6RMkjVDRZzb".into()]).unwrap();
        assert_eq!(clips.len(), 3);
        assert!(clips.iter().all(
            |clip| clip.voice_id.as_deref() == Some("JBFqnCBsd6RMkjVDRZzb")
                && clip.group_id == "family_pickup"
        ));
        assert_eq!(
            clips.iter().filter(|clip| clip.segment_index == 0).count(),
            1
        );
        assert_eq!(
            clips.iter().filter(|clip| clip.segment_index == 1).count(),
            2
        );
        assert!(payload(&data, &clips[2]).get("expected").is_none());
        let mut invalid = data.clone();
        invalid.scenarios[2].prior_segments = vec!["family_benign_details".into()];
        assert!(validate(&invalid).is_err());
        invalid = data;
        invalid.scenarios[2].split = "held_out".into();
        assert!(validate(&invalid).is_err());
    }
    #[test]
    fn selector_options_accept_small_authored_source_without_enabling_generation() {
        let args = [
            "--dataset",
            "fixtures/audio-family-development.json",
            "--scenario",
            "family_benign_details",
            "--voice-slot",
            "caller_george",
            "--voice-ids",
            "JBFqnCBsd6RMkjVDRZzb",
            "--dry-run",
        ]
        .map(str::to_owned);
        let mut selected = options(&args).unwrap();
        assert!(!selected.generate);
        assert_eq!(selected.voices, ["JBFqnCBsd6RMkjVDRZzb"]);
        let clips = selected_plan(&family_data(), &selected).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].scenario_id, "family_benign_details");
        assert_eq!(clips[0].segment_index, 1);
        assert_eq!(clips[0].voice_slot, "caller_george");
        // Full source validation still rejects invalid unselected context.
        let mut invalid = family_data();
        invalid.scenarios[0].split = "held_out".into();
        assert!(selected_plan(&invalid, &selected).is_err());
        selected.scenarios = vec!["missing".into()];
        assert!(selected_plan(&family_data(), &selected).is_err());
        selected.scenarios.clear();
        selected.voice_slot = Some("missing".into());
        assert!(selected_plan(&family_data(), &selected).is_err());
        assert!(
            options(&[
                "--scenario".into(),
                "family_opening".into(),
                "--scenario".into(),
                "family_opening".into()
            ])
            .is_err()
        );
    }
    #[test]
    fn one_voice_subset_preserves_default_source_and_request_ceiling() {
        let args = [
            "--voice-slot",
            "caller_brisk",
            "--voice-ids",
            "JBFqnCBsd6RMkjVDRZzb",
            "--scenario",
            "clinic_details",
        ]
        .map(str::to_owned);
        let selected = options(&args).unwrap();
        let data = data();
        let clips = selected_plan(&data, &selected).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].id, "clinic_details_caller_brisk");
        assert_eq!(data.voice_slots.len(), 3);
        assert_eq!(plan(&data, &[]).unwrap().len(), 36);
        let mut excessive = data;
        excessive.scenarios.push(family_data().scenarios[0].clone());
        assert!(selected_plan(&excessive, &selected).is_err());
    }
    #[test]
    fn planned_render_identity_binds_text_and_voice_but_excludes_authored_labels() {
        let voices = ["voiceone1".into(), "voicetwo2".into(), "voicethree3".into()];
        let original = data();
        let clips = plan(&original, &voices).unwrap();
        let first = serde_json::to_value(&clips[0]).unwrap();
        let alternate_voice = serde_json::to_value(&clips[1]).unwrap();
        assert_eq!(
            first["text_sha256"],
            format!("{:x}", Sha256::digest(clips[0].text.as_bytes()))
        );
        assert_ne!(first["render_sha256"], alternate_voice["render_sha256"]);
        assert_eq!(first["text_sha256"], alternate_voice["text_sha256"]);
        let mut relabeled = original.clone();
        relabeled.scenarios[0].expected.classification = "clarify".into();
        relabeled.scenarios[0].expected.rationale = "A revised rubric only".into();
        let relabeled_clip = serde_json::to_value(&plan(&relabeled, &voices).unwrap()[0]).unwrap();
        assert_eq!(first["render_sha256"], relabeled_clip["render_sha256"]);
        let mut reworded = original;
        reworded.scenarios[0].text.push(' ');
        let reworded_clip = serde_json::to_value(&plan(&reworded, &voices).unwrap()[0]).unwrap();
        assert_ne!(first["render_sha256"], reworded_clip["render_sha256"]);
        assert_ne!(first["text_sha256"], reworded_clip["text_sha256"]);
        assert!(
            serde_json::to_value(&plan(&data(), &[]).unwrap()[0]).unwrap()["render_sha256"]
                .is_null()
        );
    }
    #[test]
    fn render_identity_binds_model_format_and_voice_settings_without_rubric_leakage() {
        let data = data();
        let clip = &plan(
            &data,
            &["voiceone1".into(), "voicetwo2".into(), "voicethree3".into()],
        )
        .unwrap()[0];
        let request = payload(&data, clip);
        let original = render_hash(&request, "voiceone1", &data.output_format);
        let mut settings_changed = request.clone();
        settings_changed["voice_settings"]["stability"] = json!(0.6);
        assert_ne!(
            original,
            render_hash(&settings_changed, "voiceone1", &data.output_format)
        );
        let mut model_changed = request.clone();
        model_changed["model_id"] = json!("a_different_model");
        assert_ne!(
            original,
            render_hash(&model_changed, "voiceone1", &data.output_format)
        );
        assert_ne!(
            original,
            render_hash(&request, "voiceone1", "a_different_format")
        );
        let mut rubric_changed = request;
        rubric_changed["expected"] = json!({"classification":"risky"});
        assert_eq!(
            original,
            render_hash(&rubric_changed, "voiceone1", &data.output_format)
        );
        let reordered = json!({"voice_settings":{"use_speaker_boost":true,"style":0.0,"stability":0.5,"similarity_boost":0.75},"model_id":data.model_id,"text":clip.text});
        assert_eq!(
            original,
            render_hash(&reordered, "voiceone1", &data.output_format)
        );
    }
    #[test]
    fn stable_render_hash_matches_independent_python_compact_json_vector() {
        // Independently computed with Python json.dumps(sort_keys=True,
        // ensure_ascii=False, separators=(",", ":")) and hashlib.sha256.
        let request = json!({"text":"I'm calling from the clinic because Sarah's appointment time changed.","model_id":"eleven_multilingual_v2","voice_settings":{"stability":0.5,"similarity_boost":0.75,"style":0.0,"use_speaker_boost":true}});
        assert_eq!(
            render_hash(&request, "EXAVITQu4vr4xnSDxMaL", "mp3_44100_128"),
            "da033b2026720c4667c33f4a747c0dc9ab2f20c4015dd1e421005edb7c11bea4"
        );
    }
    #[test]
    fn dry_run_has_thirty_six_grouped_development_variants_and_no_expected_labels_in_provider_text()
    {
        let data = data();
        validate(&data).unwrap();
        let clips = plan(&data, &[]).unwrap();
        assert_eq!(clips.len(), 36);
        for voice in &data.voice_slots {
            let group = clips
                .iter()
                .filter(|c| c.group_id == "revocable_request" && &c.voice_slot == voice)
                .collect::<Vec<_>>();
            assert_eq!(group.len(), 2);
            assert!(
                group
                    .iter()
                    .all(|c| c.split == "development" && c.voice_id.is_none())
            );
        }
        let p = payload(&data, &clips[0]);
        assert_eq!(p["text"], data.scenarios[0].text);
        assert!(p.get("expected").is_none());
        assert!(p.get("acceptable_actions").is_none());
    }
    #[test]
    fn rejects_split_leakage_invalid_paths_and_cross_group_context() {
        let mut d = data();
        d.scenarios[1].split = "held_out".into();
        assert!(validate(&d).is_err());
        let mut d = data();
        d.scenarios[0].id = "../../outside".into();
        assert!(validate(&d).is_err());
        let mut d = data();
        d.scenarios[1].prior_segments = vec!["login_request".into()];
        assert!(validate(&d).is_err());
        let mut d = data();
        d.scenarios[1].segment_index = 0;
        assert!(validate(&d).is_err());
    }
    #[test]
    fn missing_key_and_voice_identity_cannot_enable_generation() {
        assert_eq!(generation_key(false, None).unwrap(), None);
        assert_eq!(
            generation_key(false, Some("configured-key".into())).unwrap(),
            None
        );
        assert!(generation_key(true, None).is_err());
        assert!(generation_key(true, Some(" ".into())).is_err());
        assert_eq!(
            generation_key(true, Some("fixture-key".into()))
                .unwrap()
                .as_deref(),
            Some("fixture-key")
        );
        assert!(plan(&data(), &["same".into(), "same".into(), "same".into()]).is_err());
    }
    #[test]
    fn atomic_output_preserves_existing_work() {
        let dir =
            std::env::temp_dir().join(format!("firstring-audio-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("clip.mp3");
        atomic_create(&path, b"original").unwrap();
        assert!(atomic_create(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn cli_defaults_to_no_network_and_has_no_call_control_mode() {
        assert!(!options(&[]).unwrap().generate);
        assert!(
            !options(&["--voice-ids".into(), "voiceone,voicetwo,voicethree".into()])
                .unwrap()
                .generate
        );
        assert!(options(&["--generate".into(), "--dry-run".into()]).is_err());
        assert!(options(&["--call".into()]).is_err());
        assert!(options(&["--generate".into(), "--limit".into(), "37".into()]).is_err());
        assert_eq!(
            options(&["--generate".into(), "--limit".into(), "1".into()])
                .unwrap()
                .limit,
            1
        );
    }
}
