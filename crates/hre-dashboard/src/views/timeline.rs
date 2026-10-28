//! Subject Directional Interaction Timeline View.

pub fn render_timeline_page() -> String {
    r##"<div class="page-header">
  <div>
    <h1 class="page-title">Subject Interaction Timeline</h1>
    <p class="page-subtitle">Directional message frequency, unanswered call streaks, and post-block contact escalation.</p>
  </div>

  <div style="display: flex; gap: 8px;">
    <input type="text" placeholder="Subject Pair Hash..." value="pair:9a2f7c..4e11" class="form-control" style="width: 200px; height: 36px; font-size: 13px;" />
    <button type="button" class="btn btn-primary" style="height: 36px;">Load Pair</button>
  </div>
</div>

<div class="card">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;">
    <div>
      <span style="font-weight: 700; font-size: 16px;">Subject A &rarr; Subject B (Directional)</span>
      <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Pair Window: 80 messages | 0 replies from recipient</div>
    </div>
    <span class="badge badge-urgent">Polite Stalker Pattern Detected</span>
  </div>

  <!-- Vertical Timeline Stream -->
  <div style="position: relative; padding-left: 28px; border-left: 2px solid var(--md-sys-color-outline-variant); margin-left: 14px;">
    
    <!-- Event 1: Initial Friendly messages -->
    <div style="margin-bottom: 24px; position: relative;">
      <div style="position: absolute; left: -35px; top: 2px; width: 12px; height: 12px; border-radius: 50%; background: var(--hre-tier-low);"></div>
      <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Day 1 &bull; 10:14 AM &bull; SMS</div>
      <div style="font-weight: 600; font-size: 14px; margin-top: 2px;">Initial Outbound Messages (12 messages)</div>
      <p style="font-size: 13px; color: var(--md-sys-color-on-surface); margin-top: 4px;">
        "Hey! Are you free today? Just wanted to catch up." (Keyword filter passes; polite tone)
      </p>
      <div style="font-size: 11px; color: var(--md-sys-color-outline); margin-top: 4px;">Recipient Reply: None &bull; Score: 18/100 (Tier: Low)</div>
    </div>

    <!-- Event 2: Volume Escalation -->
    <div style="margin-bottom: 24px; position: relative;">
      <div style="position: absolute; left: -35px; top: 2px; width: 12px; height: 12px; border-radius: 50%; background: var(--hre-tier-review);"></div>
      <div style="font-size: 12px; color: var(--md-sys-color-on-surface-variant);">Day 3 &bull; 11:42 PM &bull; WhatsApp</div>
      <div style="font-weight: 600; font-size: 14px; margin-top: 2px;">Late-Night Message Burst (34 messages unreciprocated)</div>
      <p style="font-size: 13px; color: var(--md-sys-color-on-surface); margin-top: 4px;">
        "Why aren't you answering? I saw you online. Hope you haven't forgotten me."
      </p>
      <div style="font-size: 11px; color: var(--hre-tier-review); margin-top: 4px; font-weight: 600;">
        Behavior Velocity Alert: Recipient response ratio 0.00 &bull; Score: 54/100 (Tier: Review)
      </div>
    </div>

    <!-- Event 3: Block Event -->
    <div style="margin-bottom: 24px; position: relative; background: var(--hre-tier-high-bg); padding: 12px; border-radius: 8px;">
      <div style="position: absolute; left: -47px; top: 14px; width: 12px; height: 12px; border-radius: 50%; background: var(--hre-tier-high);"></div>
      <div style="font-size: 12px; color: var(--hre-tier-high); font-weight: 700;">RECIPIENT ACTION: BLOCK REGISTERED</div>
      <div style="font-size: 12px; margin-top: 2px;">Subject B blocked Subject A on WhatsApp & Telegram</div>
    </div>

    <!-- Event 4: Post-block Contact Attempt -->
    <div style="margin-bottom: 12px; position: relative;">
      <div style="position: absolute; left: -35px; top: 2px; width: 12px; height: 12px; border-radius: 50%; background: var(--hre-tier-urgent);"></div>
      <div style="font-size: 12px; color: var(--hre-tier-urgent); font-weight: 700;">Day 4 &bull; 02:18 AM &bull; Phone / Alternate ID</div>
      <div style="font-weight: 600; font-size: 14px; margin-top: 2px;">Post-Block Persistent Contact (40 calls overnight)</div>
      <p style="font-size: 13px; color: var(--md-sys-color-on-surface); margin-top: 4px;">
        "You can't just block me like that. I know where you work."
      </p>
      <div style="display: flex; gap: 8px; margin-top: 6px;">
        <span class="badge badge-urgent">Tier: Urgent (Score: 92/100)</span>
        <span class="badge badge-built">Action: Throttle & Warn</span>
        <span class="badge badge-built">Secondary: AlertModerator</span>
      </div>
    </div>

  </div>
</div>"##
        .to_string()
}
