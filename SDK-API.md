# On-glasses WebAssembly apps

Casa Mira is bundled with the phone app. Phone startup, foreground return and
glasses connection check its installed package and install it through the SDK
when missing or different from the bundle. Matching packages do not transfer or
restart. The phone then restores the running app or opens Home. Firmware contains
no Casa bytecode, UI or product logic; installed Casa can boot offline from flash.

The phone Apps drawer installs optional apps into the same external flash store.
Installed apps survive phone disconnects and glasses reboots. Ordinary navigation
opens an installed name locally. Casa is hidden from app lists and remains the
protected Home destination.
Casa is one resident WASM controller with `shell_init`, `shell_render`, and
`shell_event`. It keeps running while one content module is switched beneath it.
Home and Settings are destinations inside that controller. Updating its installed
package replaces the resident; ordinary app switches never reload it. This is
SDK ABI 2; bundled apps migrate together and no per-app Shell path remains.

Each optional app contains a WebAssembly 1.0 module up to 160 KiB. The persistent catalog supports 30 apps including Casa, subject
to storage capacity. The partition is 3 MiB on the 32 MiB flash configuration,
and 1,216 KiB on the 16 MiB configuration. Two catalog sectors and sector-rounded
payloads consume part of that budget; updates need space for the new payload
alongside the current one. Audio, networking, AI, transcription, credentials and
HTML companions still live on the phone. Optional app downloads and updates remain explicit.

The store writes a new payload into free sectors, verifies its CRC, then publishes
a new catalog in the inactive metadata sector, writing its commit marker last.
An interrupted download or metadata write leaves the prior installation valid.
The original volatile `MWPK` app-pack path has been removed.

`build-wasm-casa.sh` builds only the phone-bundled `casa_mira.wasm`. Casa changes
ship with the phone app; no firmware resource export or firmware update is needed
unless a change requires a new generic SDK capability.
Firmware source and hardware validation details are in
[Persistent app library](../../Firmware/Halo/metaglass-ap4_Halo/boards/meta_bounds/glass/metaglass_ambiq4/mira_sdk/wasm/APP-LIBRARY.md).

The retired Bluetooth UI SDK, JavaScript runtime, HTML apps and companion
loaders are archived under `Archive/BluetoothSDK` and excluded from active
app builds and backend routing. There is no native Casa or legacy-firmware
fallback. Firmware source changes require the user's cloud firmware updater;
do not compile, package or flash firmware locally.

### Shared Casa state

Home and the dashboard above content apps share one resident dashboard cache,
subscription, visibility state, and controls. App switches preserve that feed;
hidden dashboard updates continue draining. Full IMU ownership still suppresses
look-up until the content app releases it. After building Casa, run
`node MiraApp/scripts/test-wasm-dashboard.mjs` from the repository root to check
the bundled WASM against the host's subscription replacement semantics.

`MiraPhoneServices.casaHybridTranscriptSnapshot()` preserves the existing
conversation projection: translated captions, message IDs, the latest user/AI
pair, cumulative streaming answers, confirmation state, and tool progress.
WASM receives text wrapped for its viewport through pushed field updates. The
phone retains full message history. Captions, streaming AI answers, status and
quick-skill labels publish when their source changes; the shared response cursor
also subscribes. There is no periodic BLE request for these values. A received
caption redraws on the next app tick, without waiting for the render interval.

`Shared/push_data.rs` provides reusable SDK subscriptions. Apps call `ble_request`
with method `0x0070` and a topic u16 plus mode u8, then consume the ordinary reply.
The returned request ID is also the subscription token. This declaration and its
reply are local, including while offline. Firmware owns the role (content 0,
resident 1) and app identity. Subscriptions are scoped to that owner; content
replacement preserves resident tokens. Supported phone topics are conversation
(`0x0050`, mode 1 for Casa captions/skills), assistant presentation (`0x0035`), dashboard
(`0x0020`, mode 1 when visible), languages (`0x0051`, mode 1 for source, 0 for target),
notification overlay (`0x0053`), and onboarding (`0x0056`). Mode is
otherwise zero. Initial dashboard visibility is restored by its subscription;
subsequent open/close gestures send the explicit visibility command below.

Dashboard field 8 is one byte indicating an active, user-enabled Listen session.
Casa uses it for the wake-word hint; temporary AI microphone sessions leave it off.

Assistant presentation field 12 is `1` while an active Soniox or Gemini connection
is poor, otherwise `0`. It is independent of assistant visibility and drives the
Home status-bar text: `Poor connection` replaces the `Talk to me` hint. The phone owns
latency, upload-backlog and connection checks; firmware only transports the field.
Gemini's `connection.ping`/`connection.pong` exchange measures the phone-to-bridge
round trip through the audio-forwarding loop without making a model request.

Turning Listen off from the glasses dims the background and shows a titled
box, "End listening?", with two equal buttons: End, then Keep Listening, which
is selected initially. Listening stays
active until End is selected; double tap dismisses the box without stopping.
The same confirmation covers Home, the dashboard above apps, and the glasses
Listen shortcut. Phone controls and explicit voice requests keep their existing behavior.

Each subscription starts with current values. Later updates contain only changed
fields: field u8, byte length u16, then opaque bytes; an empty value clears a field.
The phone retains only the newest pending value per field and prioritizes captions.
One batch may already be in flight. A batch is fragmented into at most 368 bytes per
RPC `0x040e`, preceded by session u32, token u32, revision u32, offset u16 and total
u16 (all integers little-endian). Logical batches are capped at 8192 bytes.
Firmware treats topics and data as opaque, with sixteen subscription slots and a
four-packet mailbox. `ble_stream_read(token, out, capacity)` drains that local
mailbox; it sends nothing over Bluetooth and still returns revision/offset/total
followed by bytes. The SDK publishes a complete batch atomically and ignores
retries. Every live subscription must be drained even when its UI is hidden.

RPC `0x040f` reconciles the complete registry without invalidating SDK tokens or
restarting apps. Its request is the nonzero host session u32. Its response is:
session u32, registry revision u32, last accepted stream revision u32, count u8,
two NUL-padded 32-byte owner names (content then resident), and `count` records of
token u32, topic u16, mode u8, owner u8. At most sixteen records fit one response.
A new session clears incomplete mailbox traffic; packets from older sessions are
rejected. The host continues stream revisions beyond the returned watermark and
publishes current state for every feed. Within the same session, unchanged feeds
keep their queues; app switches reconcile only the changed declarations.

Firmware emits one-way SDK event `0x0072` containing the registry revision when
declarations change. An uncollected change is retried at most once per second
until the host reads the registry; unchanged state emits no traffic. Phone and
watch share `WatchShared/MiraWasmStateSync.swift` for the handshake, bounded retries,
session identity and framing. Connection, foreground and stream-failure recovery
use this same path, independently of package checks and display ownership.

This protocol change requires matching current phone/watch and firmware builds;
firmware builds and deployment use the cloud updater. The guest SDK imports and
subscription calls are unchanged. Product topics and UI remain in phone-delivered
SDK apps. Hardware latency and reconnect behavior still need device validation.

Notification arrivals use phone-owned card data and action handlers. Casa owns
expiry and display state locally: it captures the initial screen state once,
selects the backdrop before drawing, and restores sleep after the card exits.
Replacements inherit that state. The phone pushes card changes to the resident WASM Shell,
which presents the card above Casa or a bundled child app. Swipes choose actions/scroll controls,
tap activates, and double tap uses the pushed AI-output state locally. A modal/card consumes double tap first, then an open sidebar, then a valid
local app claim. Otherwise the default global action interrupts active AI or
toggles the sidebar. Interrupt acknowledgements only release
transport slots and never gate navigation or other input.
Interacting stops an ordinary card's automatic dismissal. Confirmation decisions
carry the card's ID and snapshot revision; stale or repeated decisions are
rejected. Lost action replies are not automatically retried. Long bodies use
the existing scroll increments and body-line layout.

Language pickers use `SonioxTranslationLanguages` and the shared recent-first
ordering. Auto detect is source-only, followed by up to eight unique recent
languages, then the remaining catalog. Selection uses the existing validated
settings writer and `LanguagePreferencesService` persistence.

Language, overlay, and onboarding subscriptions publish field 0 as a complete
snapshot: revision u32, zero offset u16, byte count u16, then UTF-8/NUL-delimited
fields (up to 4096 bytes). `Snapshot::push_tick` assembles transport fragments
atomically using the existing snapshot buffers. The revision identifies the
card shown when an action is sent. No read, unchanged reply, or continuation
request is sent by the glasses. Conversation uses independent changed fields
through `Fields::tick` instead.

- Topic `0x0035`: Casa's single assistant presentation, owned by the phone's
  `MiraAssistantPresentation`. Field 0 is session ID, 1 output revision, 2 activity
  (`0` idle, `1` connecting, `2` listening, `3` thinking, `4` speaking, `5` disconnected),
  3 content (`0` empty, `1` answer, `2` confirmation), 4 answer text, 5 activity label,
  6 visibility (`0`/`1`), 7 monotonic turn number, 8 the NUL-separated rich-page fields
  (key, title, lead, body, style, icon), 9 the NUL-separated confirmation fields,
  10 the tool icon (`0` none, `1` search, `2` memory, `3` mail, `4` calendar,
  `5` web, `6` contact, `7` document), and 11 whether the pending confirmation
  was approved on the phone's side (`0`/`1`, as a spoken "approve" is): Casa
  then shows the same check as a tap on Approve and closes the exchange itself.
  The backend derives the icon from tool
  identity/source metadata; Casa preserves its morph animations without parsing labels.
  Metadata uses UTF-8 text. The SDK sends changed fields in one atomic batch,
  including all clears on replacement; activity changes do not resend the page.
  Casa renders this state and owns only animation, scroll and control focus.
  New content resets scrolling; activity changes and form edits preserve it.
  Dismissal closes the session, so delayed output cannot reopen it. Backend voice
  events carry `presentation_turn`; rich-page jobs retain their originating turn.
- Topic `0x0050`: shared conversation and quick-skill labels. Casa uses mode `1`,
  which sends only caption state (field 0), caption text (9) and skill labels
  (12–14). It does not select assistant content from this feed.
- Topic `0x0051`: ordered language code/name pairs. Source and target pickers
  share one subscription slot; switching mode replaces it, and the SDK keeps
  draining its local mailbox after the picker closes.
- Topic `0x0053`: ordinary notifications. Confirmations arrive only on `0x0035`.
- `0x0054`: action u8 followed by the displayed card's UTF-8 ID; `0..2`
  select an action and `255` dismisses/denies. Delayed actions cannot affect a
  replacement card.
  A confirmation is a form instead of a card: fields are id, `1`, app name,
  heading, rows, choices, approve label, deny label. Rows and choices are `0x1e`
  separated; a row is `label\tchoice count\tselected\tdetail\tvalue` and a choice
  is `label\tdetail`, grouped in row order. Detail draws dimmer under the value
  (a recipient's number). A row with choices shows its position, takes focus and
  opens a numbered list; otherwise every row is drawn the same. Picking one sends action `0x40 | row << 4 | choice`, which rewrites that
  argument on the staged tool call and leaves the form up; `1` approves and `0`
  denies. New forms focus the first selectable control in display order,
  skipping read-only rows.
- `0x0055`: interrupt active AI output. Empty request; one-byte response is `1`
  when consumed and `0` otherwise. This acknowledgement never triggers navigation.
- `0x0058`: wake Mira, sent by Casa on tap-and-hold. Same path as the spoken wake
  word. With Listen off the phone starts it for this one conversation and stops it
  when the conversation closes. Empty request; one-byte response is `1` when
  accepted and `0` when blocked or a conversation is already open.
- Topic `0x0056`: pushed onboarding state: epoch, enabled, start stage, completed voice
  token, voice failure, last accepted command token, whether response audio is active, and current phase.
  Phase 31 belongs to the separate intro app; Casa waits until it reports phase 1.
  The WASM controllers own the phase transitions.
- `0x0057`: onboarding service command: epoch u32, monotonic token u32, step u8,
  action u8, phase u8. Actions are speak (0), prepare continuous voice input (1),
  finish (4), report phase (5), or deliver a timed reminder (6). Repeated tokens are
  acknowledged without repeating effects, including the final finish command.

Ask Mira uses the Listen session. Its voice phases are hidden (0), connecting (1),
Listening (2), thinking (3), and answer (4). The shell never starts voice input
from a hold gesture; app-owned holds such as Mira Code dictation remain available.

### Onboarding

The phone account, permission and connection setup prepares Casa and Maps through
the standard SDK install flow, then opens Casa's spoken tour directly.
Ink Reel and Mira Intro used the same video; both are retired. Their source and
assets remain for reference, but firmware no longer supports movie playback.

The tour prepares hands-free spoken turns for the restaurant question,
name/hobby, personalized question, and translation language. Gemini detects each
turn boundary; no touch-bar practice or hold gesture is required. The dashboard,
Listen and translation lessons retain their prompts and the 1.5-second pause after
looking down. Double-tap does not interrupt narration. During translation Home
remains visible with only the assistant answer row hidden.

The app lesson selects Maps, narrates a nearby destination connected to the
user's interest, then launches the installed Maps SDK app. Its companion reports
actual navigation start. The phone SDK host preserves the original voice session,
waits three seconds, releases Maps audio and rearms Casa after 350 ms. The original
personalized sendoff plays over the route; drained playback returns to Home.
Debug entry stages, including Maps, retain the BLE assumed-conversation prompts.

Physical verification still requires paired glasses and an iPhone: receive a
real notification, scroll a long body, approve/deny a
tool confirmation, resolve it on the phone while its card is open, and repeat
with the phone locked and through a BLE disconnect. Run the first-run tour on
physical glasses and verify direct Watch navigation with the phone disconnected.

Modules import drawing calls from `mira`:

```text
mira.begin()                                      -> void
mira.frame_checkpoint()                           -> i32 saved
mira.frame_restore()                              -> i32 restored
mira.line(x0, y0, x1, y1, width, gray)           -> void
mira.fill_rect(x, y, width, height, gray)         -> void
mira.stroke_rect(x, y, width, height, line, gray) -> void
mira.fill_circle(cx, cy, radius, gray)             -> void
mira.stroke_circle(cx, cy, radius, line, gray)     -> void
mira.fill_round_rect(x, y, w, h, radius, gray)    -> void
mira.stroke_round_rect(x, y, w, h, r, line, gray) -> void
mira.fill_triangle(x0, y0, x1, y1, x2, y2, gray) -> void
mira.text(x, y, w, h, gray, font, align,
          lineSpacing, utf8Pointer, utf8Length)    -> void
mira.bitmap4(x, y, w, h, pixelsPointer, pixelsLen) -> void
mira.bitmap4_rotated(x, y, w, h, pixelsPointer, pixelsLen, degrees) -> void
mira.bitmap1(x, y, w, h, maskPointer, maskLen, gray) -> void
mira.present()                                    -> void
mira.now_ms()                                     -> i32
mira.display_brightness_get()                      -> i32 percent
mira.display_brightness_set(percent)               -> i32 appliedPercent
mira.display_power_get()                           -> i32 active (0 or 1)
mira.display_power_set(on)                         -> void
mira.device_status(outPointer, capacity)            -> i32 byteCount
mira.media_state(outPointer, capacity)              -> i32 byteCount
mira.media_command(command)                         -> i32 accepted
mira.imu_ready()                                  -> i32
mira.imu_threshold(aboveCdeg, belowCdeg)            -> i32
mira.imu_stop()
mira.imu_pitch_cdeg()                             -> i32
mira.imu_roll_cdeg()                              -> i32
mira.imu_yaw_cdeg()                               -> i32
mira.imu_raw_read(outPointer)                    -> i32 ready
mira.ble_request(method, payloadPointer, length)  -> i32 requestId
mira.ble_response_len(requestId)                  -> i32
mira.ble_response_read(requestId, out, capacity)  -> i32
```

`display_brightness_get` reads the glasses' current brightness. `display_brightness_set`
clamps to 0–100 and applies brightness immediately. Both work without a phone
connection or pending phone API slots.
Casa reads brightness directly for drawing and slider input, with no mirrored brightness
state or refresh hooks; phone settings snapshots do not override it. The phone reads brightness on reconnect.
The updated Casa package requires Apollo firmware containing these imports.

`display_power_get` reads local active/asleep state, including the dimmed state.
`display_power_set(1)` wakes or refreshes the idle timer; an already-active display
stays visible. On wake the LCD stays black until the first complete frame has
transferred. `display_power_set(0)` sleeps it. Casa chooses its notification
backdrop and requests wake in the same render tick, so neither a BLE acknowledgment
nor a timer guesses when the card is ready. These imports require the matching
firmware source update through the cloud updater.

Coordinates target the 640x480 canvas. `gray` is 0 (black) through 15 (white).
`begin` starts a replacement frame on the caller's retained surface; `present`
composes content then chrome. Identical primitive payloads remain in place.
Changed and removed primitives contribute their old and new bounds to a clipped
partial repaint. Each surface has its own dirty accumulator and checkpoint.
`frame_checkpoint` saves a vector/text prefix; `frame_restore` discards only that
surface's suffix. It returns zero after a canvas reset, module replacement, or
an unavailable checkpoint. Bitmap4 prefixes cannot be checkpointed. Copied
bitmap1 masks can be retained. The helper in `Shared/retained_frame.rs` manages
local content invalidation; shell changes no longer require content repainting.

`bitmap4` copies a 4-bit grayscale sprite into the retained frame using the
same native image renderer and transparency mask as the BLE SDK. Each row is
`ceil(w / 2)` bytes, with the left pixel in the high nibble; odd-width rows pad
the low nibble. Gray 15 is transparent, so use 14 for the brightest opaque
pixel. `pixelsLen` must equal `ceil(w / 2) * h`. Width is 1–640 and height is
1–480. A frame permits at most 64 bitmap calls and 131,072 total bitmap pixels,
within the shared 512-primitive limit. Each call uses one retained primitive.
Unrotated single-color images are retained as packed 1-bit masks; multicolor
and rotated images use 4-bit pixelmaps.
Invalid dimensions/lengths, exhausted budgets and unavailable image storage
skip the call; invalid guest memory traps through WAMR validation. Coordinates
are clamped to −1024…1023 and rendering clips to the canvas. There is no scaling.

`bitmap4_rotated` uses the same bitmap format, pool, validation and limits.
It rotates around the image center with the native BLE image renderer, normalizing
integer degrees into 0–359. Rotated bounds are invalidated conservatively.
The original `bitmap4` remains an unrotated wrapper. Modules using the new import
(such as Uber's spinner) require a cloud firmware update containing it; app-specific
screens and animation logic remain in WASM.

`bitmap1` copies a packed, MSB-first monochrome mask into one of sixteen retained
32×32 slots, then draws its set pixels in the supplied 4-bit grayscale color.
Each row occupies `ceil(w / 8)` bytes and `maskLen` must equal that stride times
height. It accepts widths and heights from 1–32; invalid dimensions, lengths,
or an exhausted sixteen-mask budget shared by content and the resident skip
the call. Unlike `bitmap4`, it does not use the image pool and works with `frame_checkpoint`/`frame_restore`.

Bitmap1 uses sixteen 128-byte native mask slots, separate from the bitmap4 pool.
Bitmap4 pixels and transparency masks share a 32 KiB data budget across the
resident and content surfaces, plus 2 KiB of allocator headers. Allow for both
row strides: `ceil(w/2)*h + ceil(w/8)*h`, rounded to four bytes per allocation.
An unrotated image with one opaque gray uses only `ceil(w/8)*h` bytes; its clear
bits remain transparent and its original gray is preserved. These frame masks
share the image pool and bitmap work limit, not the sixteen retained icon slots,
and prevent checkpoints just like other frame-owned images.
A full 32-piece Chess board plus Casa's cursor uses about 21 KiB including headers.
Phone-streamed image updates have an additional 8 KiB staging buffer; a target
whose pixel capacity exceeds 8 KiB is rejected before staging. WASM bitmap4 calls
copy directly into the retained pool and do not use that staging buffer.
Large imported sprites must fit the shared budget; unavailable storage skips
the draw. The preview protocol streams the retained bitmap bytes; compact
frame masks use its existing bitmap1 record format.

The shared status bar shows a phone outline with a check when the local BLE
connection flag is set and a slash when it is disconnected or not yet known.
Classic audio pairing alone does not set this indicator. This reports the BLE
link only: it does not certify that the phone host is responding or has internet.
Battery and time continue to render locally while disconnected. The icon lives
in the SDK and uses the existing `device_status` import.

`media_state` reads the glasses' local AVRCP cache immediately, without a phone
app request or request-table slot. It returns available and playing bytes followed
by NUL-terminated UTF-8 title and artist (up to 80 bytes each; use a 164-byte buffer).
No track returns `[0,0]`; insufficient capacity returns `-1` without writing.
`media_command` sends directly to the Bluetooth chip: 0 previous, 1 toggle play/pause,
2 next, 3 volume up, 4 volume down. Returns 1 if accepted for transport, 0 if sending
failed, -1 for an invalid command. Acceptance is not playback confirmation.
Use `Shared/media.rs`; sample the local cache when opening a media view and every
250 ms while visible. Metadata arrives from the player via AVRCP independently of
the Mira phone app. These imports require the matching Apollo firmware.

`device_status` reads local hardware without Bluetooth, phone requests, or API
slots. Capacity must be at least 10 bytes; success writes and returns 10:
battery percent (0–100; other values mean unavailable), charging flag, classic
Bluetooth connected flag, BLE connected flag, RTC-valid flag, hour (0–23),
minute, month (1–12), day (1–31), and weekday (Sunday=0). Invalid capacity returns
-1; WAMR validates guest memory. Battery/charging use the local battery cache;
date/time use Apollo's RTC, which continues after phone disconnection.
Connection warnings must use this local BLE flag. A slow, expired, or rejected
phone request is not evidence of disconnection. The shared shell checks the flag
when a phone action starts or fails; its five-second warning timer only controls
how long the message stays visible.

Casa and the shared app dashboard sample these values at most once per second.
Their SDK code formats battery, clock, and date even before any phone response;
phone dashboard fields cannot overwrite them. Weather/calendar still use phone
services. This import needs an Apollo update; Airoha routing is unchanged.

Orientation and screen coordinates use these conventions across SDK apps:

- Angles are centidegrees (`100` = 1 degree). Positive yaw turns right;
  compute the shortest signed yaw difference across the -180°/180° boundary.
- Positive pitch looks up. Screen X increases rightward and screen Y increases
  downward, so head-controlled screen positions use `x = centerX + yawDelta * scale`
  and `y = centerY - pitchDelta * scale`. Apply user axis-inversion settings after
  this conversion, including the inverse mapping used for edge recentering.
  Raw phone-streamed AHRS `attitude.y_pitch` has the opposite pitch sign; the
  IMU Visualizer displays those raw sensor fields. Do not copy its raw pitch sign
  into WASM head controls. Roll and acceleration vectors are sensor axes, not
  screen-coordinate deltas.

The shared SDK shell defaults to local pitch thresholds: looking up past 27°
opens the notification dashboard inside an app, and lowering to 24° closes it.
Casa and other shell apps include the same dashboard layout and text handling.
Threshold monitoring sleeps when the screen is off and resumes on screen wake;
it does not fill the motion ring or send continuous IMU data over BLE.

`imu_threshold(aboveCdeg, belowCdeg)` returns 0 below, 1 above, -1 while asleep
or awaiting calibrated data, and -2 while a full WASM or phone IMU stream owns
the sensor. Threshold values and dashboard behavior belong to the SDK.

The orientation getters and `imu_raw_read` claim full IMU sampling, which
suppresses the look-up dashboard. `imu_ready` stays zero until a calibrated
sample arrives. Call `mira.imu_stop()` (Rust: `content::imu::stop()`) to release
that claim and restore threshold monitoring without leaving the app. App exit
also releases its claims. Head-controlled apps such as Mirafold remain full-IMU
consumers; ordinary shell apps get the dashboard by default. Full mode retains
the old continuous-sampling behavior. Phone streaming independently suppresses
the dashboard until it stops. These primitives require a cloud firmware update;
the dashboard itself remains phone-delivered SDK code.

`imu_raw_read` writes nine native float32 sensor values (36 bytes):
accelerometer X/Y/Z, gyroscope X/Y/Z, then magnetometer X/Y/Z. These are
copies of the existing glasses driver outputs, with no additional SDK scaling,
axis changes, filtering, fusion, or motion calculations. Returns 1 when fresh
(at most 500 ms old), 0 when unavailable/stale, or -1 for an invalid buffer.
Only use the values when the result is 1. Reading claims local IMU sampling;
`imu_stop` or app teardown releases it. This snapshot is non-consuming and
requires no phone, BLE subscription, or calibrated attitude. Raw reads do not
request sensor fusion; orientation processing runs only for other active consumers.

Pre-rendered movie assets, the four `animation_*` imports, and their bundled
audio playback APIs are no longer supported. Apps still animate normally by
updating drawing primitives and small sprites in their own render loops.

`ble_send(0x0065, payload, 4)` is a local usage-journal command, available offline.
Payload: little-endian event u16 (nonzero), value u16. Returns 0 when enqueued,
-1 for invalid input, -2 when the journal cannot accept it. Do not retry from a
render loop. The collector adds app identity, time, and phone BLE connection state.
Shared shell codes: 1 sidebar opened, 2 sidebar closed (value: 0 app, 1 double tap,
2 selection); 3 look-up dashboard opened, 4 button-opened dashboard (value:
cached notification count, 65535 unknown). Codes 1–255 are reserved for shared
SDK behavior; custom app events should use 256–65535. This marks a UI transition,
not proof that a notification was read. Hooks belong at transitions, not draws.

For other methods, `ble_send` sends a one-way phone command with request ID zero. It allocates no
response slot; the phone executes the command without sending an API response.
Success means the command was handed to transport, not that the phone performed
it. It is not queued for reconnect or retried. Subscriptions require
`ble_request`, because they need a response token. Local UI state must not wait
for a phone command.

`ble_request` is the asynchronous escape hatch for work that belongs on the
phone, including transcription, account-backed data, and AI. Up to eight
requests may be in flight, request and response payloads are capped at 384
bytes, and unanswered requests expire after eight seconds. Poll
`ble_response_len`: `-1` means pending, `-2` unknown, and `-3` timed out. A
non-negative value is the response size and can be consumed once with
`ble_response_read`; `-4` means the supplied output buffer is too small.
Phone-side API failures are returned as `-100 - status`.

Casa uses these phone service API methods:

- `0x0001` / `0x0002` — start or stop transcription.
- `0x0003` — transcription snapshot: state byte, final byte, then UTF-8 text.
- `0x0010` — three NUL-delimited quick-skill labels.
- `0x0011` — trigger the quick-skill slot in the one-byte request payload.
- `0x0012` — AI state byte followed by UTF-8 response text.
- `0x0020` — dashboard visibility command: one byte, `1` when open, `0` when
  closed. The response has no payload. Send only when visibility changes.
  Dashboard content uses subscription topic `0x0020`: field 0 contains three
  empty placeholders (time/date/battery come from local hardware), weather,
  calendar state (`0` loading, `1` available, `2` unavailable), then up to two
  calendar title/when/place triples. Field 1 contains a reserved byte (formerly phone brightness) and a captions byte
  (always `1`) followed by NUL-delimited source/target
  language names and codes, then Mute AI (`0` or `1`). Field 2 contains transcription state and final bytes.
  Fields 3–6 contain one inbox row each: count byte, index byte, then NUL-delimited
  source, title, body, and relative time. Field 7 contains ring paired, connected,
  battery percent (`255` unknown), and charging bytes. It covers phone-owned
  JCorp and glasses-owned AIZO rings, pushes on ring state changes, and refreshes
  the battery when the dashboard opens. Only the look-up dashboard status bar
  shows a ring icon and percentage beside the glasses battery; the normal app
  status bar hides it. Unknown/disconnected power shows
  `--`, and no paired ring or no phone BLE link hides the ring indicator.
  Each dashboard field fits one transport
  packet and is applied directly to renderer state by `Events`, without keeping
  a second full copy of the dashboard. Hidden dashboards receive cleared inbox
  rows; opening publishes current rows. There are no settings or inbox read RPCs.
- `0x0022` — change a phone-owned setting;
  the first byte is the setting key (`3` for source, `4` for target) and the
  remainder is its UTF-8 language code. Key `5` sets Mute AI with exactly one
  value byte (`0` off, `1` on), sharing the phone's saved preference. The settings
  snapshot appends its state as NUL-delimited field 4 (`0` or `1`) after the four
  language fields. All other keys are rejected. Brightness
  uses the local display imports; captions are always shown while listening.
- Installed packages are queried through `app_registry(index,out,capacity)` and
  `registry_revision()`, never through a phone catalog. Index 0 is the protected
  resident; indices 1 onward are content packages. Each 40-byte record contains
  revision u32 LE, flags u32 LE (`1` launchable content, `2` protected resident),
  and a NUL-padded 32-byte printable ASCII ID/name. Return 40 means success,
  0 means end, and negative means invalid arguments or a changed revision.
  Names are stable IDs across replacement. Shell keeps only sorted indices and
  refreshes them when the store revision changes. `app_launch(id,length)` queues
  a launch by ID locally, returning 1 if accepted, 0 if rejected, -1 if invalid.
- `app_resource(index,name,name_len,out,capacity)` reads an installed module's
  named WASM custom section without launching it. It uses the same indices as
  `app_registry`. Names are 1–64 bytes. Returns the payload length, 0 if absent,
  -1 for invalid arguments, malformed data or read failure, -2 if the catalog
  changed, and -3 if the output is too small. Only the module is scanned;
  only module bytes are read. The caller checks `registry_revision`
  around a multi-app snapshot. This import requires the current SDK firmware.

### App sidebar icons

Apps define their own 32 × 32 monochrome icon in Rust, alongside their code:

```rust
#[path = "../Shared/content.rs"] mod content;
app_icon!(content::app_icon::Icon::new()
    .circle(16, 16, 12).line(10, 16, 22, 16).line(16, 10, 16, 22).0);
```

The const helper offers two-pixel lines, rectangle outlines and circles.
Alternatively pass a `[u8;128]` mask directly: four bytes per row, MSB first,
one bit per pixel. Set bits are tinted to match the sidebar row. `app_icon!`
retains the mask in a `mira.icon` custom section, including with the SDK's
`--strip-all` build flags. Define it once per module.

Casa reads the icons directly from installed packages via `app_resource`,
caches them until the catalog revision changes, and draws them next to app
names. Imported apps use the same mechanism. No phone connection or running
app instance is required. Missing or invalid masks get a generic app icon.
Firmware provides the resource reader and bitmap storage; all icon definitions,
fallbacks, sorting and sidebar rendering remain in SDK apps.

### Other phone services

- `0x0031` — open an installed app locally, keeping the shared Mira page attached. A NUL after the app name may be
  followed by an SDK-defined launch context. No BLE transfer is required.
- `mira.audio_level(channel: i32) -> i32` — local PCM loudness, 0–255.
  Channel `0` reads the microphone before LC3 encoding; `1` reads the assistant
  speaker stream after decoding at playback consumption. Values approximate
  RMS energy over 50 ms, mapped from −48 dBFS (0) to −12 dBFS (255).
  Invalid channels and readings older than 400 ms return 0; stream close also
  clears the level. Internal DSP → MCU → display SPI telemetry never goes over
  phone BLE. Apps read it during animation ticks and own their smoothing and UI.
- Topic `0x0032` exposes the SDK voice status for app consumers. Casa uses
  `0x0035` for both the sheet and the tour's narration; it does not subscribe to
  separate voice-state or rich-page feeds.
- `0x0033` — consume the launch context supplied with the most recent local
  app switch. Casa uses this to open Settings directly from a child app.
- Topic `0x0059` — pushed Home route request: field 0 is a token that changes per
  request, field 1 the route (`home` or `settings`). The phone publishes it when it re-selects
  the running Home, which the firmware does not resume; a fresh start ignores the retained
  request and takes its route from the `0x0033` launch context instead.

Font values are `0` for 24 px, `1` for 18 px, `2` for 32 px, and `3` for 40
px. Text alignment is `0` left, `1` center, or `2` right. All text is copied
into firmware-owned bounded storage before the WASM call returns.
An app must export:

```text
app_init()          -> void
app_render(elapsedMs) -> void
app_stop()          -> void
app_event(event)    -> void
```

WASM input event values are `0` click, `1` swipe forward, `2` swipe backward,
`3` double click, `9` long click, `10` hold start, and `12` hold end.
For menus and readers, event `1` selects the previous/up/left item or earlier
content; event `2` selects the next/down/right item or later content. The wire
names `scrollTop`/`scrollBottom` and touch-bar forward/backward refer to this same
pair. Phone-preview and Watch up/left gestures send `1`; down/right sends `2`.
Game-specific actions may use the pair differently (e.g. Mine Walk forward/back,
Dino jump, or Music volume up/down), but shared sidebar/card navigation always
uses the same previous/next rule. Do not invert the shared events per app.
Firmware routes every gesture to `shell_event(event) -> i32` first: 1 consumes,
0 forwards to the active content module. Shell alone imports Casa UI code from
`WasmApps/CasaMira`; content apps import `Shared/content.rs` for generic services.
Double-tap precedence is modal/notification overlay, open sidebar, active app's
valid local claim, then Shell's default global action. Double tap also requests
AI cancellation; neither phone state nor its reply delays navigation.
`gesture_policy(event,local)` accepts events 3, 9, 10 and 12 with local 0 or 1
only from the active content instance. Claims reset on app switch. Maps and
Tetris use `content::holds_local(true)` for long-click and hold edges. A claimed
hold release always reaches the app; the app checks chrome visibility before
starting a held action. Home's contextual controls are handled inside Shell.
A content module that claims hold edges owns its own held-action presentation;
Shell adds nothing beside the pointer. A conversation with Mira still opens the
shared answer sheet above it.

Casa Home uses the liquid orb presentation prototyped in the retired AI app.
This is presentation only: Casa's established hold routing, conversation state,
confirmation/onboarding gates, notification behavior and quick-skill APIs remain
in charge. The shared cursor dissolves into a particle-sphere orb while
Casa reports held input, thinking or active output: 140 dots on a rotating sphere
that slowly breathes between a hollow halo and a filled ball. It follows the shared
cursor position and glide on every frame, including during release and playback,
then morphs back at the current hover target. It has no separate movement state.
Near display edges, the center moves only enough to fit the loudest halo. The orb uses the existing
realtime topic's input level while held and output level afterward; stale samples
expire after 400 ms. The shared glasses meter uses the transport's normalized
loudness directly, without boosting quiet speech, with a smoothed 140 ms
attack / 220 ms release that grows the sphere from 20 to 30 px, pushes it out to a
halo, adds up to 15% radial shimmer and speeds its spin. Rim dots are brightest
and largest in the halo, near-side dots in the ball, over a dark backing disc. Playback timing remains
an estimate from the phone's LC3 pacer, without firmware playback timestamps.

Listen and available insight buttons form one bottom row, ending at y=477.
There is no Ask Mira hold gesture. Soniox mentions of “Mira” or “mirror” wake a
fresh Gemini session with the previous 10 seconds of buffered microphone audio.
Mira has one presentation on every screen: the answer sheet. It rises from the
bottom edge over Home, Settings and any content app alike, with the same
geometry, timing, gestures and timeout; there is no cursor bubble beside an
app's pointer. Over a content app the Shell draws the sheet on its chrome
surface, shades the app to the sidebar's level, reports it as covered through
`content_context` bit 1, and blocks the app pointer. Opening the sheet closes
the sidebar and notification dashboard. The dashboard stays closed while the
sheet is visible; after dismissal, looking down and up reopens it. The guided
tour keeps its narration bubble.
On Home the sheet covers the captions and controls, with the orb centered on its
top border. The AI transcript comes first, followed by rich output underneath in
the same continuous column. Swipes smoothly scroll both; one more forward swipe
at the end dismisses the sheet and closes Gemini, stopping playback. Double-tap
does the same from anywhere. Soniox remains armed for the next wake word.
Dismissal and timeout slide the sheet out with the same 240 ms easing as its entrance.

Pending confirmations replace the previous transcript and rich output, reset
scrolling, and occupy the sheet on their own. Late reply updates cannot replace
a pending form. They reuse notification content without the outer card border,
bell icon, or app-name header. The orb becomes the selection
cursor for the Deny/Approve buttons and long-text controls. Swipes select confirmation
controls, bringing them into view; taps activate them. Approving draws a check
in the sheet for about a second before it lowers; Cancel lowers it at once.
Pending confirmations pause
the reply timeout. Double-tap cancels the confirmation and closes the conversation.
Confirmations use the sheet inside content apps and Settings too, and lower an
open dashboard so they never appear as a floating card. Ordinary notifications
retain their top overlay.

The sheet crown renders the phone-selected activity label beside the orb.
Listening is hidden during generation and playback and returns when microphone
input is available, while the last answer remains visible. One 25-second inactivity timer owns
both the sheet and Gemini: it pauses during generation/playback, renews on new
output, and closes both on expiry. Scrolling does not change the timer. Empty or declined wakes use the
same timer; there is no separate follow-up deadline. Closing the voice session
for other reasons does not clear an existing reply. A fixed ring in the upper-right loses segments as time runs out, matching notifications.
Status and countdown draw above the retained content frame, so their animation
does not re-layout the rich reply. Settings, the sidebar, notifications and the look-up dashboard
retain their established controls and voice behavior.

Build with `scripts/build-wasm-casa.sh`.
The orb uses a fixed particle count and bounded triangle discs, never per-pixel WASM rasterization.
The separate AI bundle is removed; the existing withdrawn-app cleanup removes
previously installed bundled copies on reconnection, preserving imported apps.

The host retains two independent primitive surfaces in one bounded arena:
content is composed first and chrome last. The existing 512 primitive slots,
64 text slots, four 1-bit mask slots and image workspace are shared, not doubled.
Each surface retains its own ordered slot map, frame/checkpoint state and dirty
bounds. Updating one surface cannot truncate or reset the other. Repaint uses
only the union of changed old/new bounds; unchanged primitive frames do not
trigger a redraw. A content module draws only its own layer.

`content_cursor(x,y,flags)` publishes a target; bit 0 makes the content pointer
visible. Clear it on surfaces without a focused control, such as navigation.
The resident picks up target and visibility changes on its next local tick;
cursor animation does not depend on BLE or content redraw frequency.
Sidebar, dashboard, and notification controls show their
own pointer. Menu transitions glide to and from the app's anchor during AI
calls as well as while idle.
`content_info(field)` returns active-content presence (0), valid double-tap
claim (1), cursor x/y/flags (2/3/4), opaque Shell context (5), last host error
(6), or valid local gesture bitmask (7, bit `1 << event`).
`content_id(out,capacity)` copies the active ID and returns its byte length,
or 0 at Home. Only the resident may call `content_context(flags)` to publish
occlusion hints; Casa defines bit 0 as sidebar and bit 1 as overlay/dashboard.
Firmware does not interpret those UI meanings.
Only the resident may call `content_shade(level)`: the compositor then draws
every retained content primitive with its color scaled by `level/15` (15 is
unchanged, 0 is off) without the app redrawing. Fractional palette levels round
up so nonblack details stay visible at nonzero shade levels; black stays black. Chrome is unaffected, and 4bpp
images keep their raw pixels. The runtime restores 15 whenever content stops.
Casa shades content to 2 while its sidebar or a notification card is visible
(cards that hide the background paint the frame themselves), and Home, which
the resident paints itself, scales its own colors to the same level.

The host sends a generic `0x0071` launch notification with the active ID to the
phone so companions follow launches even when content never makes a BLE request.
Content stop releases only its own requests, subscriptions, images and input
policy. The resident's conversation, overlays, icons and global gesture routing
continue in the same execution context.

Each callback has a firmware-enforced instruction budget. WASI, threads,
filesystem, sockets, and dynamic code generation are intentionally unavailable.
Modules with a WebAssembly start section or legacy `__post_instantiate` hook are
rejected; put initialization in the metered `app_init` export instead.

The firmware runtime also disables the bulk-memory and reference-types
proposals. Keep Rust builds on the flags used by the checked-in build scripts;
in particular, both `bulk-memory` and LLVM's separate `bulk-memory-opt`
code-generation feature must be disabled so slice copies do not emit
`memory.copy` (`0xfc 0x0a`).

The repository includes thirteen reference experiences. Build their
binaries with:

```sh
MiraApp/scripts/build-wasm-tetris.sh
MiraApp/scripts/build-wasm-dino.sh
MiraApp/scripts/build-wasm-jelly-bounce.sh
MiraApp/scripts/build-wasm-geo-dash.sh
MiraApp/scripts/build-wasm-showcases.sh
MiraApp/scripts/build-wasm-casa.sh
MiraApp/scripts/build-wasm-mirafold.sh
MiraApp/scripts/build-wasm-mine-walk.sh
MiraApp/scripts/build-wasm-physics-games.sh
```

Tetris controls on the glasses are: tap to rotate and swipe forward/backward to
move left/right. Double-tap is reserved for the shared Casa sidebar. Opening
Tetris again starts a fresh game.

Dino Run advances automatically. Tap or swipe to jump; use the same gestures
after a collision to restart. Double-tap opens the shared Casa sidebar.

Jelly Bounce is a scrolling platformer built to showcase fluid, physics-driven
vector animation. Look left or right to move (the further you turn, the faster
the jelly runs), nod up or tap to jump, and swipe forward or backward to dash
left or right, respectively. Its 16-point soft-body membrane runs an 8 ms spring-and-volume
solver: impacts, jumps, acceleration, dashes, and pickups send ripples through
the blob while a 60 Hz-ready render path redraws every host frame. The jelly
also throws particle splashes, collects droplets, activates checkpoints, and
reforms after a collision. If fused orientation is unavailable, swipes set its run direction.
Landing pads are broad, moving platforms carry the jelly, and neutral head input
brakes promptly in the air. The first checkpoint has a clear runway; later spike
groups are smaller and collision follows the teeth with a forgiving inner body.
Six courses ramp the difficulty: levels four to six add spiked balls that patrol
floors, bob over landing pads, or orbit the gaps on a chain, plus platforms that
slide sideways and carry the jelly. Every course is verified solvable on the
real physics. The title screen is a level picker: swipe to choose any level and
tap to play; levels cleared in the current session show a tick. The game uses
only the on-glasses SDK and keeps no state between launches. It is an optional
install from the phone Apps drawer.

Jumping Jam (developer app) is an auto-running rhythm platformer in the style of Geometry Dash.
The cube runs right at a fixed speed per level; nod up or tap to jump, and
swipe to jump as well. Spikes and block faces kill, block tops carry the cube,
and every death explodes the cube and restarts the level from the beginning
with the next attempt number after a short pause. Jumping is one rule: a quick
upward flick of the head (4° within 160 ms) jumps, with a 300 ms cooldown so a
single nod fires once. There is no baseline, arming state, or hold behaviour. A jump
buffer and a short coyote window absorb IMU latency. The cube spins while
airborne and snaps to a quarter turn on landing; a progress bar and percentage
sit at the top. Levels are one byte per tile (`.` ground, `^` spike, `1`-`3`
block heights, `A`-`C` spiked blocks). An optional parallel `roof` string marks
rocket tiles (`0` bare ceiling, `1`-`6` hanging blocks, `v` ceiling spike): a
portal swaps the cube for a rocket that flies a nine-tile corridor. Holding the
head above or below its resting pitch sets the climb rate (1.5° dead zone, full
300 px/s at 9°); the resting pitch tracks slowly in cube mode and freezes in
rocket mode. The rocket slides along the floor, ceiling and block tops and dies
on block faces and spikes; without the IMU a tap flaps against gravity. The
title screen is a level picker: turn the head left or right past 10° (or
swipe) to step through five courses of rising speed and tap to play; the turn
re-arms within 4° of rest and repeats every 700 ms while held. Levels 4 and 5
mix tighter cube patterns with rocket sections. Cleared levels show a tick. Double tap or long click returns to the picker; double tap
from the picker opens the sidebar. The first three courses are composed
from patterns whose jump windows were measured on the real integer physics
with a buffered-input solver, and no forced jump has a window under 136 ms;
levels 4 and 5 were solved on the same physics with inputs limited to a 64 ms grid.
The game uses only the on-glasses SDK and keeps no state between launches.
Enable developer apps to install it.

The additional showcase experiences demonstrate different parts of the ABI:

- **Casa Mira** — a WASM reimplementation of the regular glasses shell that
  also draws the AI's visual pages (`CasaMira/visual_page.rs`): when the
  backend composes a structured page beside a spoken answer, the phone pushes
  it as part of topic `0x35`. Swipes scroll the answer sheet; the page
  is read-only, and double tap dismisses the exchange. It is a WASM
  reimplementation of the regular glasses shell that
  mirrors the BLE SDK `CasaMira.html` layout and the native Casa chrome:
  status bar with translation badge, the Listen toggle, wrapped live
  captions, the request bubble and answer thread, three phone-backed quick
  skills beside Listen, a gliding cursor, the settings
  page with brightness slider and translation pickers, the sidebar, and the
  look-up dashboard with date, weather, calendar, media
  controls, and wrapped notifications. Text is wrapped and ellipsized inside
  the module with the firmware glyph advance tables, because the firmware
  paints each row literally. Double-tap opens or closes the sidebar; looking
  up (27°) opens the dashboard and looking down (below 24°) closes it.
  The sidebar slides in from the left edge (240 ms ease-out) and slides back
  out on close or app switch. It is a card hugging the left edge with rounded
  right corners, sized to its rows rather than the full display height, holding
  one scrolling list of 32 px rows without
  a header or row borders: Mira, Settings, then installed apps with the three
  most recently opened first, newest at the top, and the rest alphabetically.
  Seven rows are visible; the window follows the focused row, which is full
  ink while other rows are a darker shade. A dot marks the running
  destination. Recents are resident state and reset with Casa.

- **Mirafold** — MIRA rendered in one centered row of four close-set animated
  letters. Swipe between letters and tap to cycle variants; head movement bends
  the focused letter. Folding joints, twisting ribbons, flowing ink trails, and
  opening fans animate continuously at a target 30 fps, with no extra control
  panel. The 13 variants include pixel, turtle, ribbon, catenoid, planetary,
  Pong, rough, griddy, fan, and string studies. Double-tap opens the shared sidebar.

- **Breakout** — real-time collision physics, a dense 48-brick scene, and fused
  IMU input. Look left or right to move the paddle; the launch heading becomes
  center. Tap to launch/pause. Double-tap opens the shared sidebar.
  Swipes remain available if calibrated orientation is unavailable.
- **Starfall** — a hands-free auto-firing space shooter. Small yaw/pitch motions
  steer, close dodges charge overdrive, and every third sector brings a boss
  with weapon and hull rewards. Play starts automatically; look down to pause
  and hold left to retry. Tracking loss, notifications, and
  the shared sidebar freeze play. Tap/swipes provide manual controls without
  IMU.
- **Tether** — a two-player co-op obstacle course with an elastic rope. Turn/look
  to steer, time moving hazards, and open a pressure-switch gate.
  Both players reach the exit together and retry hands-free. Install/Open Tether
  from Mira's built-in app list. Its bundled SDK HTML companion creates/joins a
  room through standard SDK messaging and WebSockets. No game-specific Swift service or firmware is
  needed. Disconnects, tracking loss, and shared overlays pause play. See
  [Tether](Tether/README.md) for the package, deployment, and verification.
- **Mine Walk** — a first-person voxel landscape with raised tree canopies,
  bark, leaf, grass, dirt, stone, sand and water textures, distance shading, and
  clipped 3D faces. Stand still for about one second to calibrate footsteps, then
  step to move 0.72 blocks in the heading measured at the impact. Motion records
  are buffered independently of rendering. Swipes remain manual forward/back
  steps; tap recenters, long-click pauses/resumes walking, and double-click
  resets. Movement sweeps a player radius, blocks tree trunks, and climbs at most
  one block. Terrain remains procedural with no fixed edge. The phone WASM
  preview's Step button injects an acceleration waveform through the same ABI.
  The frame caps geometry plus texture to 470 triangles with room for the HUD;
  textures prioritize large visible surfaces within that fixed display budget.
  Scene construction is split into bounded callbacks while motion processing
  continues; the camera is held consistent for each build and only complete
  frames are published. This preserves the 500,000-instruction callback limit.

Developer Settings also exposes a glasses runtime log. It reads the bounded
Apollo-side event ring over the existing Mira BLE RPC and reports upload/CRC,
WAMR initialization, module linking, first-frame primitive counts and bounds,
input events, IMU activation/readiness, shutdowns, and the callback stage for
any trap.

The connected-device **Glasses Preview** is hardware-backed for a running
WASM app. While that screen is open, the phone pages the retained WAMR vector
display list from Apollo (`0x040B`) and paints those exact primitives in the
normal preview canvas. Physical touch-bar input, resident app switches, and
IMU-driven frames therefore appear without running a second copy of the module.
Gestures on the phone preview are injected into the same on-glasses runtime
through `0x040C`. The mirror is built to stay off the link: each poll first asks
for a header-only page (`slot == total`) and downloads nothing while the present
generation is unchanged, polls back off from 120 ms to 1 s as the display rests,
bitmaps already seen (matched by the record's size and hash) are skipped by
resuming the slot at its end, and reads use the bulk lane so OS and app traffic
go first. Polling stops when the preview closes or the app leaves the foreground.

The same Developer Settings card has **Preview WASM on Phone**. It executes
the selected bundled binary in a hidden WebKit WebAssembly runtime and renders
the resulting display list with the existing glasses preview canvas. Touch
gestures map to the normal input ABI, orientation buttons simulate pitch/yaw,
and deterministic local BLE fixtures exercise transcription, AI, skills, and
dashboard responses without connected glasses. This is a functional layout
and behavior simulator; firmware instruction metering, memory pressure, BLE
latency, and the optical appearance of the waveguide still require hardware.

The shared assistant hold and transcription start controls show “Connect your phone and open Mira” when the local BLE phone link is unavailable. Blocked actions are not queued for reconnection. Request failures or timeouts do not imply disconnection; the prompt is gated only by the local BLE flag. Existing phone internet and service errors remain phone-owned.

Ring taps, swipes, hold-start, and release use the same local input events as the
touch bar. AIZO delivers these directly from the Bluetooth MCU to Apollo; its phone
copy is telemetry only. JCorp/X5B must remain phone-based. The phone forwards gesture
codes in receive order without interpreting holds or starting AI. WASM owns gesture
behavior and requests phone services only for actions that need them.


### Public companion device access

All installed HTML/WASM companions, including imported apps, can call:

- `await miraCompanion.getMediaState()` returns `{available, title, artist,
  playing, elapsedMs, durationMs, volume}`. Volume is the phone output level in
  percent; elapsed/duration are milliseconds. The read uses the current Bluetooth
  now-playing service. `available: false` means no metadata was returned.
- `await miraCompanion.mediaCommand(code)` accepts the same codes as the local
  WASM media API: 0 previous, 1 toggle, 2 next, 3 volume up, 4 volume down. Returns
  `{accepted}`; transport acceptance is not playback confirmation.
- `await miraCompanion.getIMU()` starts/reuses glasses streaming and returns
  `{available, active, source, sampleCount, receivedAt, ageMs, accelerometer,
  gyroscope, magnetometer, attitude}`. Missing vectors/fields must be treated as
  unavailable. Vectors contain x/y/z and sensor timestamp metadata. Attitude uses
  x_roll/y_pitch/z_yaw and can include a stale flag. These are the existing raw
  phone-stream units/conventions (attitude degrees), distinct from the WASM
  orientation getters' centidegrees. Use the local WASM IMU getters for games
  that need minimal latency. Poll raw diagnostics at a bounded rate (e.g. 400 ms).
- `await miraCompanion.stopIMU()` releases that companion's streaming claim.
  Stop, app switching, and companion replacement also release it automatically.
- `await miraCompanion.startCapture()` starts or shares the glasses microphone
  and returns `{sampleRate: 16000, channels: 1, format: "s16le"}` when ready.
  Repeated starts reuse the capture. Startup rejects if glasses are unavailable,
  microphone enable fails, or the capture is stopped before it becomes ready.
- `miraCompanion.onCapture(handler)` registers a PCM handler and returns an
  unsubscribe function. Each frame is `{samples: Int16Array, sampleRate: 16000,
  channels: 1, sampleOffset}`. Samples are signed 16-bit mono, decoded from the
  glasses' LC3 stream with the phone's microphone gain applied. Delivery batches
  contain roughly 100–256 ms of audio. The host buffers at most 256 ms and drops
  the oldest waiting samples when WebKit falls behind; `sampleOffset` identifies
  the first sample since capture started, so gaps reveal dropped samples.
  Service-level microphone mute suppresses delivery. Audio stays on the phone;
  use the companion to process it or upload it to your own service.
- `await miraCompanion.stopCapture()` releases only the companion's microphone
  claim and returns `{ok: true}`. It discards pending audio and leaves other
  microphone consumers active. Unsubscribing removes a handler without stopping
  capture. App switching, page replacement/termination, and glasses disconnect
  also release capture; after reconnect, call `startCapture()` again.

Register the handler before starting capture:

```js
const unsubscribe = miraCompanion.onCapture(({samples, sampleOffset}) => {
  // Process signed PCM samples here; use sampleOffset to detect delivery gaps.
});
await miraCompanion.startCapture();
// When finished:
await miraCompanion.stopCapture();
unsubscribe();
```

These methods require the companion's app to be running. They reuse existing
phone hardware services and require no new firmware imports. They are available
through the same host bridge for every package; there are no app-name privileges.
`sendToApp` returns a promise that resolves when the host accepts the message,
not when WASM consumes it; rejection can be used for bounded retry/backpressure.

For phone text entry, `miraCompanion.bindPasteButton(button, onPaste)` displays an
iOS system Paste control in the HTML button's bounds. The HTML button remains a
hidden layout anchor; the existing web editor is unchanged. `onPaste(text)`
receives plain text from the user's paste action, with multiple text items joined
by a blank line. The app decides where to insert it. This avoids programmatic web
clipboard reads and their additional confirmation. Hidden/disabled button state
and bounds are synchronized automatically. Multiple Paste controls are supported.
The binding exposes `ready` and `dispose()`; controls are also removed on page
replacement. This phone capability requires no glasses connection or WASM change.

`Shared/companion.rs` accepts optional compile-time input/output buffer sizes:
`Companion::<512, 512>::new(APP_NAME)`. Defaults retain the full 4096-byte buffers.
The input capacity must fit the mailbox's sequence field and the entire incoming
JSON message, not just the decoded application payload. Size messages to fit
these app-chosen bounds and the public 3500/4096 byte limits.

The five ordinary packages and their build workflow are described in
`SDKApps/README.md`.


For compact realtime state, use `companion::Realtime::new(APP_NAME)` and
`miraCompanion.setLatestForApp(value)`. `send` replaces pending controls (JSON up
to 379 bytes); call `tick` every frame. The host retains only the latest scene
(JSON up to 380 bytes), and returns it in the same BLE round trip as the controls.
`0x0063` operation 2 carries an input ID (u32 LE) followed by JSON; its response
is a scene revision (u32 LE) followed by JSON. Old/duplicate input IDs are ignored within a registered stream.
The helper registers the companion via `0x0064`, limits requests to one in
flight, and returns changed scenes only. Pending realtime messages in the phone
host are coalesced; ordinary ordered messages are preserved. Use this channel
for replaceable state, never transactions or events that must all be delivered.
Tether demonstrates this path and three-second head-pose calibration.

Phone request tokens use bit 30 for resident ownership and the low 30 bits for
a nonzero sequence. Tokens remain positive i32 values. Content switches invalidate
content requests on both sides while resident requests and subscriptions survive.

### Rich-output icons

Casa draws a compact 60-name catalog in `fastapi_backend/app/ai/glasses_visual.py`
as distinct Lucide vectors covering places, travel, people, communication,
news, education, fitness, security, technology, work, finance and everyday needs.
Keep this catalog small: every entry adds code to the resident app's RAM footprint.
The editor prompt advertises the same complete catalog. Unknown wire names use
the help icon; the editor chooses the closest core symbol or leaves it empty.

`CasaMira/visual_icons.rs` is generated from pinned `lucide-static@1.45.0`
(ISC / Feather MIT; attribution is in `CasaMira/visual-icons.LICENSE`). The
generator snaps to the 24 px grid and simplifies subpixel corner arcs. A packed
path stream in code constants avoids a bitmap atlas and fits the existing
128 KiB resident memory and 160 KiB executable budgets. Content apps retain
their 64 KiB memory limit. The firmware runtime pool reserves 288 KiB for the
resident module, one content module, and interpreter overhead; its separate
24 KiB free-heap guard remains in place. All strokes use Home's shading
wrappers; no bitmap-mask slots or firmware changes are needed.

Casa prepares rich-page text once after measuring it, retaining wrapped bytes,
text widths, table columns, and detail-card metrics in resident memory. Scrolling
reuses those values; vertical viewport changes do not reflow the fixed-width
column. Content changes invalidate the cache even when a page ID is reused.
Installing the 128 KiB Casa build requires firmware with the 288 KiB runtime
pool; firmware compilation and deployment use the cloud updater workflow.

Regenerate after changing the canonical catalog:

```sh
npm install --prefix /tmp/mira-visual-icon-tools --ignore-scripts --no-audit --no-fund lucide-static@1.45.0 svgpath@2.6.0
node MiraApp/scripts/gen-visual-icons.mjs /tmp/mira-visual-icon-tools
bash MiraApp/scripts/build-wasm-casa.sh
```

`CasaMira/visual-icons.json` records the source geometry for review and catalog
parity checks; it is not shipped into WASM linear memory. The icon check runs
the generated renderer at 24/32/36 px, checks representative families for
distinct geometry, and enforces the device module-size limit. The rich-page
check additionally renders dense pages with the most complex icons.
