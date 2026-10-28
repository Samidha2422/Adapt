# Dashboard bridge

API owner: implement `hre_dashboard::backend::DashboardBackend` and layer
`axum::Extension(DashboardState(Arc::new(bridge)))` onto the merged dashboard
router before serving. Existing `router<S>()` remains generic. No self HTTP or
credentials are used by this crate. Missing injection returns 503.

Exact contract lives in `src/backend.rs`:

- `alerts() -> DashboardFuture<'_, Vec<AnalyzeResponse>>`: durable alert assessments.
- `health() -> DashboardFuture<'_, DashboardHealth>`: core engine health and model health.
- `metrics() -> DashboardFuture<'_, Option<DashboardMetrics>>`: measured evaluation
  summary with source/scope, sample count, simulation disclosure and optional values.
  Return `None` if no evaluated artifact is available; offload artifact IO.
- `review(Uuid, ReviewRequest) -> DashboardFuture<'_, ()>`: persist moderator review;
  return 404 for unknown alerts and 501 if unsupported. Never acknowledge a no-op.

`DashboardFuture<'a,T>` is a boxed Send future yielding `Result<T, StatusCode>`.
Dashboard detail currently selects by alert_id from `alerts()`.

Protect every `/dashboard` route except embedded static assets with moderator auth.
The API owner supplies browser login/session handling: an opaque, expiring HttpOnly
SameSite=Strict cookie (Secure on HTTPS), server-side validation, logout/revocation,
and same-origin validation on cookie-authenticated mutations. Bearer remains usable
for tools. Client API keys must not authorize dashboard access. Do not put tokens in
HTML, URLs, JavaScript, or browser storage. Apply no-store to private pages/fragments.
SSE `/v1/stream/alerts` and dashboard fragment `/dashboard/queue/items` need the same
session auth. SSE carries JSON; the dashboard refreshes server-rendered rows instead
of inserting JSON into HTML. Review forms use the dashboard bridge.

Health reports use backend/degradation from the engine. Hash verification is shown
only as artifact integrity, never as proof of model execution. Heuristics do not
imply trained inference. Evaluation values are unavailable until supplied by bridge.

Verification is delegated: logged format/check recipes, dashboard tests, then an
authenticated browser smoke check of overview, queue/SSE, detail/review and metrics.
Host tests must cover anonymous/client-key denial, cookie/bearer moderator access,
CSRF denial, session expiry, and missing/failed bridge behavior.

## Current API gaps

- Persisted reviews need an implementation before `review()` can return success.
- Supply measured report mapping to `DashboardMetrics` or return `None`.
- Recording POST accepts metadata and synthesizes audio instead of accepting bytes;
  the dashboard disables that upload until a real multipart contract exists.
- Job history listing and pair timeline are not in this bridge. Job rows are
  unavailable; the retained timeline is explicitly a simulated example.
- Slang candidate/lexicon examples remain explicitly simulated local previews;
  a typed candidate/lexicon bridge is needed to replace them with durable data.
- Scenario buttons are reference controls only. The logged seed recipe remains the
  execution path; the dashboard no longer fabricates completion acknowledgments.

CSV now uses JSON `{platform, conversation_id, csv, draft:false}`. Evidence export
uses JSON `{subject_id}`; verification submits the complete export JSON. Dates
were removed because the current export contract does not filter by date. Browser
API requests use the parent session and never store credentials or upload contents.
