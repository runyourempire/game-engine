use crate::codegen::CompileOutput;
use super::util::{html_escape, format_time, calc_timeline_duration};

pub(super) fn build_timeline(output: &CompileOutput) -> String {
    if output.arc_moments.is_empty() {
        return String::new();
    }
    let duration = calc_timeline_duration(&output.arc_moments);
    let dur_fmt = format_time(duration);

    // Moment dots on the rail
    let mut moment_dots = String::new();
    let mut moment_btns = String::new();
    for m in &output.arc_moments {
        let pct = if duration > 0.0 {
            (m.time_seconds / duration) * 100.0
        } else {
            0.0
        };
        let time_str = format_time(m.time_seconds);
        let label = m.name.as_deref().unwrap_or(&time_str);
        moment_dots.push_str(&format!(
            r#"<div class="timeline-moment" style="left:{pct:.1}%" data-time="{time}" title="{label}"><span class="timeline-moment-label">{label}</span></div>"#,
            pct = pct,
            time = m.time_seconds,
            label = html_escape(label),
        ));
        let btn_fallback = format!("@{}", time_str);
        let btn_label = m.name.as_deref().unwrap_or(&btn_fallback);
        moment_btns.push_str(&format!(
            r#"<button class="timeline-moment-btn" data-time="{time}">{label}</button>"#,
            time = m.time_seconds,
            label = html_escape(btn_label),
        ));
    }

    format!(
        r##"<div class="timeline" id="timeline" data-duration="{duration}">
  <div class="timeline-controls">
    <button class="timeline-play" id="tl-play" title="Play/Pause">&#9654;</button>
    <div class="timeline-rail" id="tl-rail">
      <div class="timeline-track"></div>
      <div class="timeline-fill" id="tl-fill"></div>
      <div class="timeline-head" id="tl-head"></div>
      {moment_dots}
    </div>
    <span class="timeline-time" id="tl-time">0:00 / {dur_fmt}</span>
  </div>
  <div class="timeline-moments">{moment_btns}</div>
</div>"##,
        duration = duration,
        moment_dots = moment_dots,
        dur_fmt = dur_fmt,
        moment_btns = moment_btns,
    )
}

pub(super) fn build_shortcut_overlay(has_timeline: bool) -> String {
    let mut rows = String::new();
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">1</span><span class="shortcut-desc">Preview tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">2</span><span class="shortcut-desc">WGSL tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">3</span><span class="shortcut-desc">API tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">4</span><span class="shortcut-desc">Editor tab</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">Ctrl+S</span><span class="shortcut-desc">Save to disk</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">S / M / L</span><span class="shortcut-desc">Component size</span></div>"#);
    if has_timeline {
        rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">Space</span><span class="shortcut-desc">Play / Pause</span></div>"#);
        rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">&larr; / &rarr;</span><span class="shortcut-desc">Step -1s / +1s</span></div>"#);
        rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">Home</span><span class="shortcut-desc">Go to start</span></div>"#);
    }
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">X</span><span class="shortcut-desc">Toggle x-ray mode</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">P</span><span class="shortcut-desc">Toggle pixel autopsy</span></div>"#);
    rows.push_str(r#"<div class="shortcut-row"><span class="shortcut-key">?</span><span class="shortcut-desc">Toggle this overlay</span></div>"#);

    format!(
        r#"<div class="shortcut-overlay" id="shortcut-overlay" onclick="toggleShortcuts()">
  <div class="shortcut-box" onclick="event.stopPropagation()">
    <h3>Keyboard Shortcuts</h3>
    {rows}
  </div>
</div>"#,
        rows = rows,
    )
}
