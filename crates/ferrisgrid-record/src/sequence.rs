use crate::reducer::CheckpointReason;
use ferrisgrid_core::{
    ActionKind, AgentAction, ErrorKind, FerrisError, Result, ScreenInfo, parse_action_block,
    render_action_block,
};
use std::fs;
use std::path::Path;

const STEP_START: &str = "<!-- ferrisgrid-step:start -->";
const STEP_END: &str = "<!-- ferrisgrid-step:end -->";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextMode {
    Redacted,
    Plain,
    Off,
}

impl TextMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Redacted => "redacted",
            Self::Plain => "plain",
            Self::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "redacted" => Ok(Self::Redacted),
            "plain" => Ok(Self::Plain),
            "off" => Ok(Self::Off),
            other => Err(FerrisError::new(
                ErrorKind::Protocol,
                format!("unsupported text mode: {other}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceScreen {
    pub screen_id: String,
    pub fingerprint: String,
    pub logical_width: u32,
    pub logical_height: u32,
}

impl From<&ScreenInfo> for SequenceScreen {
    fn from(screen: &ScreenInfo) -> Self {
        Self {
            screen_id: screen.screen_id.clone(),
            fingerprint: screen.display_fingerprint.clone(),
            logical_width: screen.logical_width,
            logical_height: screen.logical_height,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceStep {
    pub number: u32,
    pub occurred_at_ms: u64,
    pub started_at_ms: u64,
    pub action: AgentAction,
    pub checkpoint: Option<CheckpointReason>,
    pub before_frame: Option<u32>,
    pub after_frame: Option<u32>,
    pub redacted: bool,
    pub omitted: bool,
    pub external_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    pub source_session: String,
    pub text_mode: TextMode,
    pub screens: Vec<SequenceScreen>,
    pub steps: Vec<SequenceStep>,
}

impl Sequence {
    pub fn replayable(&self) -> bool {
        !self.steps.is_empty()
            && self
                .steps
                .iter()
                .all(|step| !step.redacted && !step.omitted)
    }

    pub fn render(&self) -> String {
        let mut out = String::from("## FerrisGrid Sequence\n");
        out.push_str("- schema_version: 1\n");
        out.push_str(&format!("- source_session: {}\n", self.source_session));
        out.push_str(&format!("- text_mode: {}\n", self.text_mode.as_str()));
        out.push_str(&format!("- replayable: {}\n", self.replayable()));
        for screen in &self.screens {
            out.push_str(&format!(
                "- screen: {} fingerprint={} logical={}x{}\n",
                screen.screen_id, screen.fingerprint, screen.logical_width, screen.logical_height
            ));
        }
        for step in &self.steps {
            out.push_str(&format!("\n{STEP_START}\n"));
            out.push_str(&format!("## Step {:06}\n", step.number));
            out.push_str(&format!("- occurred_at_ms: {}\n", step.occurred_at_ms));
            out.push_str(&format!("- started_at_ms: {}\n", step.started_at_ms));
            out.push_str(&format!(
                "- checkpoint: {}\n",
                step.checkpoint
                    .map(|value| value.as_str())
                    .unwrap_or("none")
            ));
            out.push_str(&format!(
                "- before_frame: {}\n",
                optional_frame(step.before_frame)
            ));
            out.push_str(&format!(
                "- after_frame: {}\n",
                optional_frame(step.after_frame)
            ));
            out.push_str(&format!("- redacted: {}\n", step.redacted));
            out.push_str(&format!("- omitted: {}\n", step.omitted));
            out.push_str(&format!(
                "- external_state: {}\n",
                step.external_state.as_deref().unwrap_or("none")
            ));
            out.push_str("\n```text\n");
            if step.redacted {
                let length = match &step.action.kind {
                    Some(ActionKind::Type { text }) => text.chars().count(),
                    _ => 0,
                };
                out.push_str(&format!(
                    "status: action\naction: type\ntext: <redacted>\ntext_length: {length}\n"
                ));
            } else if step.omitted {
                out.push_str("status: action\naction: type\ntext: <omitted>\n");
            } else if let Some(action) = &step.action.kind {
                out.push_str(&render_action_block(action, step.action.wait_after_ms));
            }
            out.push_str("```\n");
            out.push_str(&format!("{STEP_END}\n"));
        }
        out
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let parent = path.parent().ok_or_else(|| {
            FerrisError::new(ErrorKind::Storage, "sequence path has no parent directory")
        })?;
        fs::create_dir_all(parent)?;
        let temp = parent.join(format!(
            ".{}.tmp",
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("sequence.md")
        ));
        fs::write(&temp, self.render())?;
        fs::rename(temp, path)?;
        Ok(())
    }

    pub fn read(path: &Path) -> Result<Self> {
        parse_sequence(&fs::read_to_string(path)?)
    }
}

fn optional_frame(frame: Option<u32>) -> String {
    frame
        .map(|value| format!("{value:06}"))
        .unwrap_or_else(|| "none".to_string())
}

pub fn parse_sequence(markdown: &str) -> Result<Sequence> {
    let header = markdown.split(STEP_START).next().unwrap_or(markdown);
    let source_session = header_value(header, "source_session")?.to_string();
    let text_mode = TextMode::parse(header_value(header, "text_mode")?)?;
    let screens = header
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- screen: "))
        .map(parse_screen)
        .collect::<Result<Vec<_>>>()?;
    let mut steps = Vec::new();
    for section in markdown.split(STEP_START).skip(1) {
        let body = section.split(STEP_END).next().ok_or_else(|| {
            FerrisError::new(
                ErrorKind::Protocol,
                "sequence step is missing its end marker",
            )
        })?;
        steps.push(parse_step(body)?);
    }
    if steps.is_empty() {
        return Err(FerrisError::new(
            ErrorKind::Protocol,
            "sequence contains no action steps",
        ));
    }
    Ok(Sequence {
        source_session,
        text_mode,
        screens,
        steps,
    })
}

fn parse_screen(value: &str) -> Result<SequenceScreen> {
    let mut fields = value.split_whitespace();
    let screen_id = fields.next().ok_or_else(|| {
        FerrisError::new(ErrorKind::Protocol, "sequence screen is missing screen_id")
    })?;
    let fingerprint = named_field(fields.clone(), "fingerprint")?;
    let logical = named_field(fields, "logical")?;
    let (logical_width, logical_height) = logical.split_once('x').ok_or_else(|| {
        FerrisError::new(
            ErrorKind::Protocol,
            "sequence screen logical size is invalid",
        )
    })?;
    Ok(SequenceScreen {
        screen_id: screen_id.to_string(),
        fingerprint: fingerprint.to_string(),
        logical_width: parse_u32(logical_width, "logical_width")?,
        logical_height: parse_u32(logical_height, "logical_height")?,
    })
}

fn named_field<'a>(fields: impl Iterator<Item = &'a str>, name: &str) -> Result<&'a str> {
    fields
        .filter_map(|field| field.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
        .ok_or_else(|| {
            FerrisError::new(
                ErrorKind::Protocol,
                format!("sequence field is missing {name}"),
            )
        })
}

fn parse_step(body: &str) -> Result<SequenceStep> {
    let number = body
        .lines()
        .find_map(|line| line.trim().strip_prefix("## Step "))
        .ok_or_else(|| FerrisError::new(ErrorKind::Protocol, "sequence step number is missing"))?;
    let action_block = body
        .split("```text\n")
        .nth(1)
        .and_then(|value| value.split("```").next())
        .ok_or_else(|| FerrisError::new(ErrorKind::Protocol, "sequence action block is missing"))?;
    let redacted = parse_bool(step_value(body, "redacted")?, "redacted")?;
    let omitted = parse_bool(step_value(body, "omitted")?, "omitted")?;
    let action = parse_action_block(action_block)?;
    Ok(SequenceStep {
        number: parse_u32(number, "step")?,
        occurred_at_ms: parse_u64(step_value(body, "occurred_at_ms")?, "occurred_at_ms")?,
        started_at_ms: parse_u64(step_value(body, "started_at_ms")?, "started_at_ms")?,
        checkpoint: CheckpointReason::parse(step_value(body, "checkpoint")?),
        before_frame: parse_optional_frame(step_value(body, "before_frame")?)?,
        after_frame: parse_optional_frame(step_value(body, "after_frame")?)?,
        redacted,
        omitted,
        external_state: match step_value(body, "external_state")? {
            "none" => None,
            value => Some(value.to_string()),
        },
        action,
    })
}

fn header_value<'a>(body: &'a str, key: &str) -> Result<&'a str> {
    markdown_value(body, key, "sequence header")
}

fn step_value<'a>(body: &'a str, key: &str) -> Result<&'a str> {
    markdown_value(body, key, "sequence step")
}

fn markdown_value<'a>(body: &'a str, key: &str, context: &str) -> Result<&'a str> {
    let prefix = format!("- {key}: ");
    body.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .ok_or_else(|| FerrisError::new(ErrorKind::Protocol, format!("{context} is missing {key}")))
}

fn parse_optional_frame(value: &str) -> Result<Option<u32>> {
    if value == "none" {
        Ok(None)
    } else {
        parse_u32(value, "frame").map(Some)
    }
}

fn parse_u32(value: &str, key: &str) -> Result<u32> {
    value
        .parse()
        .map_err(|_| FerrisError::new(ErrorKind::Protocol, format!("{key} must be an integer")))
}

fn parse_u64(value: &str, key: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| FerrisError::new(ErrorKind::Protocol, format!("{key} must be an integer")))
}

fn parse_bool(value: &str, key: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(FerrisError::new(
            ErrorKind::Protocol,
            format!("{key} must be true or false"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisgrid_core::{ActionStatus, MouseButton};

    #[test]
    fn sequence_round_trips_plain_and_redacted_steps() {
        let sequence = Sequence {
            source_session: "demo".to_string(),
            text_mode: TextMode::Plain,
            screens: vec![SequenceScreen {
                screen_id: "screen-1".to_string(),
                fingerprint: "display-1".to_string(),
                logical_width: 1512,
                logical_height: 982,
            }],
            steps: vec![SequenceStep {
                number: 1,
                occurred_at_ms: 20,
                started_at_ms: 10,
                action: AgentAction {
                    status: ActionStatus::Action,
                    kind: Some(ActionKind::Click {
                        screen_id: Some("screen-1".to_string()),
                        x: 500,
                        y: 500,
                        button: MouseButton::Left,
                    }),
                    wait_after_ms: None,
                    confidence: None,
                    reason: None,
                },
                checkpoint: Some(CheckpointReason::Click),
                before_frame: Some(1),
                after_frame: Some(2),
                redacted: false,
                omitted: false,
                external_state: None,
            }],
        };
        assert_eq!(parse_sequence(&sequence.render()).unwrap(), sequence);
    }
}
