[![Lazurite Hero](/stormlightlabs/lazurite/raw/main/docs/images/hero.png)](/stormlightlabs/lazurite/blob/main/docs/images/hero.png)

Lazurite is a cross-platform Bluesky client built with Flutter and Dart using Material You (M3) design.

## Features

[](#features)

### Home Feed & Composer

[](#home-feed--composer)

| Home Feed | Composer | Profile |
| --- | --- | --- |
| [![Home Feed](/stormlightlabs/lazurite/raw/main/docs/images/home-feed.png)](/stormlightlabs/lazurite/blob/main/docs/images/home-feed.png) | [![Compose Screenshot](https://camo.githubusercontent.com/94e1695a0f2b75466919a9653d0bd2c03249c38f18439d0853ff5f31b053f018/68747470733a2f2f706c616365686f6c642e636f2f343030783830302f3443414635302f4646464646463f746578743d436f6d706f73652b53637265656e73686f74)](https://camo.githubusercontent.com/94e1695a0f2b75466919a9653d0bd2c03249c38f18439d0853ff5f31b053f018/68747470733a2f2f706c616365686f6c642e636f2f343030783830302f3443414635302f4646464646463f746578743d436f6d706f73652b53637265656e73686f74) | [![Profile](/stormlightlabs/lazurite/raw/main/docs/images/profile.png)](/stormlightlabs/lazurite/blob/main/docs/images/profile.png) |
| View your personal timeline with support for threads and media. | Create new posts with rich text and media attachments. Supports replies and quoting. | View detailed actor profiles, including their feed and metadata. |



### Search & Profile

[](#search--profile)

| Search | About | DevTools |
| --- | --- | --- |
| [![Search Results](/stormlightlabs/lazurite/raw/main/docs/images/search.png)](/stormlightlabs/lazurite/blob/main/docs/images/search.png) | [![About](/stormlightlabs/lazurite/raw/main/docs/images/about.png)](/stormlightlabs/lazurite/blob/main/docs/images/about.png) | [![DevTools](/stormlightlabs/lazurite/raw/main/docs/images/dev-tools.png)](/stormlightlabs/lazurite/blob/main/docs/images/dev-tools.png) |
| Discover people and posts across the Bluesky network. | About (showing Rose Pine Moon theme) | Built-in logs and developer utilities for exploring the AT Protocol (Rose Pine Dawn). |



### Offline Support & Drafts

[](#offline-support--drafts)

Local-only drafts and caching powered by Drift (SQLite).

*   **Drafts:** Save posts locally and publish later.
*   **Search History:** Persisted local search history.
*   **Saved Feeds:** Manage and pin your favorite feeds.

## Architecture

[](#architecture)

### Stack

[](#stack)

*   **Framework:** Flutter (M3)
*   **State Management:** `flutter_bloc`
*   **Database:** Drift (SQLite)
*   **Networking:** Dio + `atproto`/`bluesky` packages
*   **Navigation:** `go_router`
*   **Data Serialization:** `freezed` + `json_serializable`

### Directory Structure

[](#directory-structure)

The project follows a feature-first architecture layered with a core module:

*   `lib/core/`: Shared infrastructure, database, router, and themes.
*   `lib/features/`: Feature-specific logic (Auth, Feed, Search, Profile, etc.).
    *   `<feature>/bloc/`: Business logic components.
    *   `<feature>/presentation/`: UI screens and widgets.
    *   `<feature>/data/`: (Optional) Feature-specific repositories or models.

### Data Flow

[](#data-flow)

*   **Network:** Authenticated requests are routed through user PDS; public reads use the public AppView.
*   **Database:** Drift manages local persistence for accounts, cached profiles/posts, settings, and drafts.

### Routing

[](#routing)

Lazurite uses `StatefulShellRoute` for persistent bottom navigation.

| Path | Description |
| --- | --- |
| `/login` | Authentication gateway |
| `/` | Home Feed tab |
| `/search` | Search tab |
| `/profile` | Current user profile tab |
| `/settings` | Global settings |
| `/compose` | Root-level modal for new posts |



## Local Development

[](#local-development)

Use `just` for common tasks:

*   `just format` - Runs `dart format`
*   `just lint` - Proxies `flutter analyze`
*   `just test` - Executes the `flutter test` suite
*   `just gen` - Triggers `build_runner` for code generation
*   `just check` - Runs format, lint, and tests in sequence

For a quick start:

flutter pub get
just gen
flutter run

## Database Schema

[](#database-schema)

Powered by **Drift**, the following tables are currently implemented:

| Table | Purpose |
| --- | --- |
| `accounts` | Local storage for session and auth tokens (DID, handle, service) |
| `cached_profiles` | Cached profile metadata to reduce network calls |
| `cached_posts` | Cached post content for offline viewing |
| `saved_feeds` | Locally Managed feed preferences |
| `search_history` | Persistent query history |
| `drafts` | Offline-first post drafting with media support |
| `settings` | Key-value application configuration |



## References

[](#references)

*   [Bluesky API Documentation](https://docs.bsky.app/)
*   [AT Protocol Specification](https://atproto.com/)
*   [Flutter Documentation](https://flutter.dev/docs)

## Credits

[](#credits)

*   Typography inspiration from [Anisota](https://anisota.net/) by [Dame.is](https://dame.is).
*   Custom theming inspired by [Witchsky](https://witchsky.app/).
*   DevTools (AT Protocol Explorer) inspiration from [pdsls](https://pds.ls/)
*   AT URI links pass through [aturi.to](https://aturi.to/)