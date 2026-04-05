As the title suggests, the theme of this dev log is “Software as Fan Art.” Everything I’ve worked on in February and March has been inspired by some other piece of software that I appreciate.

## Writer

For the past month or so I’ve been working on a bunch of tauri apps, each with a different front-end framework. The first one I want to talk about is Writer. It’s inspired by iaWriter in that it’s basically my attempt at remaking iaWriter with tools and libraries I know how to use. I also added some features that are a bit like Things, specifically the quick capture system. 

It’s built with codemirror, React, and React-PDF on the front-end. One of the big things I tried to attempt with this project was to use Rust as much as possible to hold state, though you’ll see that there are more lines of code in TypeScript, likely due to markup. There’s a ton of markup and code splitting because I opted to use a lint rule that forced me to keep my JSX nesting under 3-4 levels, aptly called `react/jsx-max-depth` (I use this in the SolidJS project I’ll discuss later too).

[![](https://substackcdn.com/image/fetch/$s_!q5oV!,w_1456,c_limit,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F411147f5-9ff4-48ab-b62a-cd6fc6409c7e_3080x1950.png)](https://substackcdn.com/image/fetch/$s_!q5oV!,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F411147f5-9ff4-48ab-b62a-cd6fc6409c7e_3080x1950.png)

Tauri holds document and file state and emits file-system events, and partial rendering of markdown. This is done via Tauri’s `Emitter` struct and the `notify` crate (it’s cross-platform, just like Tauri). Through JavaScript bindings, the listens for events and renders the current state of all files. An experiment I tried out with this was to build a system of ports, named after and inspired heavily by the Elm Architecture (TEA). Ports are bindings that Elm code uses to call JavaScript, and similarly, the ports in my project invoke Rust code. The basic flow is pretty simple:

> Send a message (command identified by a string), receive an update from the “back-end,” and then update the UI accordingly.

One crazy thing I discovered about Tauri while working on this is that Tauri’s external drag being enabled completely breaks HTML5 drag and drop. I spent days trying to figure out how to properly compute drop zones before finding the [github issue](https://github.com/tauri-apps/tauri/issues/14373) that explained this. I made a small website at [writer.stormlightlabs.org](https://writer.stormlightlabs.org/) and releases will be hosted on Github. I’m working on finalizing version 0.2.0 and am excited to share it.

Source code here: [https://github.com/stormlightlabs/writer](https://github.com/stormlightlabs/writer)

## Agent V

Agent V (v for visualizer) is a CLI and desktop GUI that ingests logs and session data from AI programming agents/assistants and displays them. Right now it supports Codex, Claude, OpenCode, and Crush. It’s been interesting to take a peek into how I use agents and the estimated token costs of what I ask them to do. It would be interesting to see how much it costs the companies hosting the models and the carbon footprint of my work. 

[![](https://substackcdn.com/image/fetch/$s_!TgKl!,w_1456,c_limit,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F9615f506-c9cc-413f-826d-f526b9e85a7c_2290x1450.png)](https://substackcdn.com/image/fetch/$s_!TgKl!,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F9615f506-c9cc-413f-826d-f526b9e85a7c_2290x1450.png)

As you can see from the screenshot that there are still some data quality kinks to work out. Those have been hard to keep up with. This is unconfirmed but it seems like OpenCode switched to a SQLite database to store session data while I was working on this (or I just didn’t notice it). As far as OpenCode goes, I had started with the export command from OpenCode’s CLI by having the system listen for updates to the log directory. When a log file was added it would call export and store the session. Like Writer, it uses notify as its filesystem watcher to look for new sessions in all sessions so that you can watch your sessions in real-time.

Source code here: [https://github.com/stormlightlabs/agentv](https://github.com/stormlightlabs/agentv)

## Thunderus

Thunderus is an AI agent and harness I’m working on that is meant to provide me with a first-class experience using Kimi K2.5 and GLM5. I find that using Claude and OpenCode, though effective doesn’t work perfectly. GLM with OpenCode in particular will leave off a closing brace and enter a “death loop” trying to find and fix the problem. I don’t yet know if that’s a model problem or a harness problem but I know that side loading GLM keys into Claude doesn’t cause this. I want to leave Claude as-is with Anthropic models so I stick to OpenCode despite the occasional headache. 

This is being built with Rust, specifically ratatui. I like Ratatui a lot but compared to libraries like Ink and Bubbletea which borrow patterns from frontend focused tools, it’s been a bit of a learning curve for me. Writing tools and streaming requests between the client and model hasn’t been too bad but getting the UI to match the designs I made has been difficult. One thing about designs: HTML and ASCII mockups are the way to go for TUIs.

[![](https://substackcdn.com/image/fetch/$s_!WtlS!,w_1456,c_limit,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F2a5a8741-d731-4362-99ac-10635078a92c_2290x1450.png)](https://substackcdn.com/image/fetch/$s_!WtlS!,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F2a5a8741-d731-4362-99ac-10635078a92c_2290x1450.png)

They’ll look prettier than what comes out in your terminal but they do look cool.

It’s not open source…yet but once it’s usable I’ll post an update!

## Video Editor X

Why it’s called X I don’t know. It’s basically untitled. This is the SolidJS + Tauri project I mentioned earlier. I quite like some of the constructs in Solid. It markets itself as a framework for fine grained reactivity but I really think it provides you with more transparency about the computations being done in your projects. The eslint plugin does a good job of telling you when you’re not properly leveraging signals too. Svelte obscures signals unless you try to type `createRawSnippet` where in order to use the props you pass into the snippet, you have to call a function. The big thing Solid provides as far as DX goes is no dependency array “footguns.” It automatically detects dependencies much like `$derived` an `$effect` in Svelte (which by the way, can be a footgun if you don’t use `untrack` appropriately) so the compiler reads calls to `createEffect` and `createMemo` to construct a list of dependencies. Pretty cool stuff.

[![](https://substackcdn.com/image/fetch/$s_!yhk_!,w_1456,c_limit,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F4c52b0dc-4eef-42b8-a9f9-ec5673111a1f_2290x1450.png)](https://substackcdn.com/image/fetch/$s_!yhk_!,f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2F4c52b0dc-4eef-42b8-a9f9-ec5673111a1f_2290x1450.png)

The core problem this project this solves for me is wanting to take advantage of ffmpeg and its powerful set of features. I want a simple editing setup and a way to draw audio visualizations using it with a GUI. I also want to be able to take markdown slides and turn them into simple video with audio voiceover. It takes advantage of browser APIs exposed by WebViews for audio recording, editing and parsing libraries that JS can provide like codemirror, as well as htmltocanvas for markdown to html to png for the backend to put together in a video.

Source code here: [https://github.com/stormlightlabs/video-editor](https://github.com/stormlightlabs/video-editor)

## Closing Thoughts

I think I’m the poster child for what AI provides engineers, as well as what it exposes in us. My productively has skyrocketed and I’m building a lot of stuff that I’m proud of, but I’m awful at handling releases and promoting my own work. My workflow has changed around this where I don’t power through implementation at the same rate and instead work on a set of tasks with some level of TDD, then iterate on a feature until I feel ready to move forward. This feels like slowing down but my output is turning out better. I’m also leaning into desktop and terminal projects because I don’t need to setup and configure hosting for these projects. They very easily adhere to my local-first ethos.
