# std: minimal CLI parsing driven by struct fields #30677

[https://codeberg.org/ziglang/zig/issues/30677](https://codeberg.org/ziglang/zig/issues/30677)

## Description

Migrated from [https://github.com/ziglang/zig/issues/24601](https://github.com/ziglang/zig/issues/24601)

Now that [#30644](/ziglang/zig/issues/30644) landed, this issue is easier to accomplish.

Josh Wolfe seems to have given up, at least for the time being, on tackling this issue. Is anyone else interested in taking on the mantle?

## Comments

### dotcarmen

2026-01-06T17:17:28+01:00

edit: PR is up [#30725](https://codeberg.org/ziglang/zig/pulls/30725). The PR looks a little different than what's in this comment

👋 I'd be interested in taking this on

after running through the GitHub issue and PR, and considering recent changes to `std`, here's my suggestion (using `tools/docgen.zig` as the example):

```zig
const std = @import("std");

const Args = struct {
    pub const arg0 = "docgen";
    pub const description = "Generates an HTML document from a docgen template";

    named: struct {
        @"code-dir": [:0]const u8,
        pub const help = .{
            .@"code-dir": "Path to directory containing code example outputs", // defaults to ""
        };
    },
    positional: struct {
        input: [:0]const u8,
        output: [:0]const u8,
    }
};

pub fn main(init: std.process.Init) !void {
    const arena = init.arena.allocator();
    const stderr = std.debug.lockStderr(&.{});
    const args = std.cli.parse(Args, allocator, init.minimal.args.iterateAllocator(allocator), .{
        .writer = stderr.file_writer,
        .exit = true,
    }) catch |err| switch (err) {
        error.OutOfMemory => return err,
        // these are automatically handled when `.exit = true` in options, so this is strictly optional
        error.Help => std.cli.printHelpAndExit(Args, stderr.file_writer),
        error.Usage => std.cli.printUsageAndExit(Args, stderr.file_writer),
    };

    // ...
}
```

the cli module would thus look like:

```zig
pub const ParseError = error{ Help, Usage };

pub const Options = struct {
    /// Writer for printing errors that result in `printUsage`, as well as usage and help messages.
    /// defaults to stderr if this is null
    terminal: ?std.Io.Terminal = null,
    /// Whether to automatically exit the process in the event of `error.Help` or `error.Usage`.
    exit: bool = false,
};

pub fn parse(
    Args: type, 
    allocator: std.mem.Allocator, 
    args: std.process.Args.Iterator, 
    options: Options,
) (ParseError || std.mem.Allocator.Error)!Args {
    // ...
}

pub fn printHelp(Args: type, writer: *std.Io.Writer) !void {
    // ...
}

pub const help_exit_code = 0;
pub fn printHelpAndExit(Args: type, writer: *std.Io.Writer) noreturn {
    // ...
}

pub fn printUsage(Args: type, writer: *std.Io.Writer) !void {
    // ...
}

pub const usage_exit_code = 1;
pub fn printUsageAndExit(Args: type, writer: *std.Io.Writer) noreturn {
    // ...
}
```

Notable differences from original issue and PR:

*   `std.cli.Options`:
    *   in the issue description, it had 2 fields: `help: bool` and `print_errors: bool`
    *   in the PR, it had 3 fields: `writer: ?*std.Io.Writer`, `prog: ?[]const u8`, `exit: ?bool`
    *   `prog` is now a declaration on `Args` named `arg0`
        *   KISS principle: only use `Options` in `parse`, everything that needs `prog` accepts `Args` anyways, `prog` is usually the same thing always and can be determined at comptime, keeps the name localized to the `Args` struct, and if the user *does* decide they want multiple ways of parsing arg0, then they probably want multiple `Args` structs anyways...
        *   also, it's renamed `arg0` because `prog` is ambiguous. i'm not opposed to `program_name` but that's also ambiguous and might result in the user intuitively putting `My Program` instead of `my_program`
    *   it's now `exit: bool = false` to match `.terminal`'s for improved clarity. it was null in the PR to allow usage in other functions, but we don't need that anymore
*   `std.cli.@"error"` is now split into 4 functions:
    *   `printHelp` prints the help message of the program to the given writer
    *   `printHelpAndExit` calls `printHelp` and then `std.process.exit(0)`. Notably, its signature denotes `noreturn`
    *   `printUsage` prints the usage message of the program to the given writer
        *   note that `parse` should already have printed which errors caused usage to be printed if `Options.writer != null`
    *   `printUsageAndExit` calls `printUsage` and then `std.process.exit(1)`. Also returns `noreturn`
*   `Args`:
    *   named argument help messages are now in a declaration `help` on the `named` type
        *   in the PR, this was originally done by concatenating field identifiers with `-help`. as mentioned, this will probably be removed from the language
        *   in the issue comments, mlugg suggested using an anonymous struct, which i like - it allows type safety and typo detection (possibly both in a follow-up PR?)
    *   not shown above, `Args` can also provide its own `help` and `usage` strings. This would replace auto-generated help/usage message if provided
    *   `Args` may provide `arg0` as described above (instead of `.prog` in `Options`)

edit: updated to reflect a suggestion from matklad to flatten the `Args` namespace [andrew vetoed this](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9641471)

### matklad

2026-01-06T17:24:13+01:00

For named vs positional split, we've recently switched to the following API in TigerBeetle:

```zig
const CLIArgs = struct {
    events_max: ?usize = null,

    @"--": void,
    fuzzer: FuzzersEnum,
    seed: ?u64 = null,
};
```

Stuff before `@"--": void` is named, stuff after is positional. This looks weird, but sort-of makes sense, and the use-site of CLIArgs isn't polluted by named/positional distinction.

### dotcarmen

2026-01-06T17:34:08+01:00

i like that - it's worth noting i've seen a CLI or 2 (can't remember off the top of my head) that handle positionals differently before and after `--`. i don't think it's worth supporting that use-case, but it's certainly a trade-off to consider

i'll go ahead with that design unless further discussion brings me back to `named`/`positional` stuff

### JanBeelte

2026-01-06T18:12:32+01:00

Hello!  
 I might be willing to take a stab at it.  
 For now I managed to revive and upgrade the old parser from Josh here:  
 [https://codeberg.org/JanBeelte/zig/src/branch/cli](https://codeberg.org/JanBeelte/zig/src/branch/cli)

Next steps:

*   Upgrading all cli's in the codebase

is anyone aware if there were still features missing on the [old PR](https://github.com/ziglang/zig/pull/24881) ?

### JanBeelte

2026-01-06T18:14:44+01:00

[@dotcarmen](/dotcarmen) looks like we started in parallel. If you want we can join forces 😄

### Tomcat-42

2026-01-06T18:18:45+01:00

What about taking a `getopt`-ish approach to allow users to reorder arguments? (Apologies if this was already discussed on GH; I’m still catching up on the thread).

I’ve been using variations of [`git.sr.ht/~phugen/util@e7598c6eab/item/src/util/getopt.zig`](https://git.sr.ht/~phugen/util/tree/e7598c6eab9df3e45db08c3b001922e65134b681/item/src/util/getopt.zig) in my toy projects and it works well:

*   **Without subcommands:** Argument permutation is ideal—e.g., allowing `ls dir/ -la`.
*   **With subcommands:** Stopping at the first positional argument is usually best to identify the subcommand. This also naturally enforces the restriction that flags appearing after a subcommand belong strictly to that subcommand.

Also, is automatic help/usage generation strictly necessary? I think this is better served by application-specific code (controlling formatting and exit behavior). This is straightforward to handle manually after parsing:

```zig
const Args = struct {
    help: ?void = null,
};

const args = std.cli.parse(Args, &it);
if (args.help) |_| help();

...

fn help(...) noreturn {
// pretty and app-specific description
}
```

### dotcarmen

2026-01-06T18:32:52+01:00

> looks like we started in parallel. If you want we can join forces 😄

i'm ok with that, but i'm not sure how you'd want to split up work. the simplest method is one of us works on the PR, the other adds review comments. up to you, i've been afk the last hour but was just about to start working on this based on my comment above

> What about taking a getopt-ish approach to allow users to reorder arguments? (Apologies if this was already discussed on GH; I’m still catching up on the thread).

I think the design from the original issue is somewhat settled. I don't wanna stir the waters with a new design and trigger a whole new design discussion around high-level features. I intentionally followed the previous design discussion with my last comment.

> Also, is automatic help/usage generation strictly necessary?

Of course not. But it's better to have it than to not since it greatly reduces developer overhead.

edit: also note that the solution i proposed allows for the application-specific code you desire

### pancelor

2026-01-06T22:22:32+01:00

I'm also interested! I spent some time yesterday revisiting an old half-done PR that I never submitted. I probably should have said something, but I think it's not a problem to have two separate attempts going. I'll try to keep my work as shareable as I can make it (small commits, etc)

Some design thoughts:

*   I'm keeping to the original proposal ([https://github.com/ziglang/zig/issues/24601](https://github.com/ziglang/zig/issues/24601))
*   How would you model an arbitrary number of positional args like `ls file1.txt file2.txt file3.txt`? The model of "positional" args with names seems helpful for avoiding bugs (`args.positional[4]` versus `args.positional.output`), but that model doesn't work here. I see a few options:
    *   We could allow `.positional` to be either a `struct` or a `[]const []const u8`, although that feels too magical and doesn't allow a mix of named-positionals and unnamed-positionals.
    *   We could collect trailing/unnamed positional args, separate from the named struct. (but its a bit tricky deciding how to name it and autogenerating help text)
    *   We could declare that this case is out-of-scope for this minimal arg parser.
    *   We could avoid named-positional args altogether (back to `positional: []const []const u8`). This seems less helpful and also make generating help text a bit harder.
*   It feels odd to call `std.process.exit` based on an `.exit` config flag. It'd be nice to instead let `std/start.zig` catch `error.StdCliHelp` and `error.StdCliUsage` and gracefully print help text there. But that's not currently possible, since `std/start.zig` has no idea what the shape of your `Args` is -- this would need more integration with the juicy main proposal ([https://github.com/ziglang/zig/issues/24510](https://github.com/ziglang/zig/issues/24510))

Here's a modified version of [@dotcarmen](/dotcarmen)'s example, which I used mainly to nail down my thoughts on help text:

```zig
const std = @import("std");

const OutputFormat = enum { txt, pdf, html };
const Args = struct {
    pub const program_name = "docgen"; // optional, defaults to arg[0]
    pub const description = "Generates a document from a docgen template";
    pub const epilog = "Exit status 123 for all errors";
    named: struct {
        format: OutputFormat,
        @"code-dir": [:0]const u8 = "",
        verbose: bool = false,
        thread_count: usize = 1,

        pub const help = .{
            .format: "Which document format to generate",
            .@"code-dir": "Path to directory containing code example outputs",
        };
    },
    positional: struct {
        input: [:0]const u8,
        output: []const u8, // note: either string type is allowed

        // a single list-of-strings field is allowed.
        // if present, extra positional args are allowed and collected here. must be the last field.
        // note: this is orthogonal to handling `--` properly.
        //   for instance, `docgen -- foo` would set `.input` to `"foo"`, and `extra_files` would be empty.
        extra_files: []const []const u8,
        
        pub const help = .{
            .input = "Path to the input file",
            .output = "Path to the output file",
        };
    },
};

test "docgen --help" {
	var aw: std.Io.Writer.Allocating = .init(arena);
	try std.cli.printUsage(Args, &aw.writer);
	const actual = aw.written();

	try std.testing.expectEqualStrings(
		\\Usage: docgen --format FORMAT [--code-dir CODE-DIR] [--verbose] INPUT OUTPUT [EXTRA_FILES...]
		\\
		\\Generates a document from a docgen template
		\\
		\\named arguments:
		\\  --format ENUM          Which document format to generate (txt|pdf|html)
		\\  --code-dir STRING      Path to directory containing code example outputs (default: "")
		\\  --[no-]verbose         (default: no-verbose)
		\\  --thread_count USIZE   (default: 1)
		\\
		\\positional arguments:
		\\  INPUT             Path to the input file
		\\  OUTPUT            Path to the output file
		\\  [EXTRA_FILES...]
		\\
		\\Exit status 123 for all errors
	, actual);
}

pub fn main(init: std.process.Init) !void {
    const arena = init.arena.allocator();
    const stderr = std.debug.lockStderr(&.{});
    const args = try std.cli.parse(Args, arena, init.minimal.args.iterateAllocator(arena), .{
        .writer = stderr.file_writer,
        .exit = true,
    })
    // ...
}
```

### dotcarmen

2026-01-06T22:53:11+01:00

[@pancelor](/pancelor) it looks like the only difference between yours and the one i initially proposed was `arg0` => `program_name` and the recognition of `epilog` :)

i've also *just* updated my comment using the flattened namespace from [@matklad](/matklad). otherwise, i think `epilog` is a reasonable addition. i still disagree that `program_name` is better than `arg0` for reasons i already mentioned, but that could be discussed more once a PR is opened.

i've been working on this PR for a couple of hours now using [@JanBeelte](/JanBeelte)'s rebased commit from the original PR (though i'm refactoring a fair amount as well - since I hadn't heard from Jan i went ahead and started running with it). i'm getting close to a mostly-updated (to my original comment) and working state

### dotcarmen

2026-01-06T23:07:13+01:00

to address your list:

> How would you model an arbitrary number of positional args like ls file1.txt file2.txt file3.txt? The model of "positional" args with names seems helpful for avoiding bugs (args.positional\[4\] versus args.positional.output), but that model doesn't work here. I see a few options:

the original PR slightly modified the design to accept a struct for positionals. after matklad's field flattening, that just means all fields declared after `@"--": void` are positionals

> It feels odd to call std.process.exit based on an .exit config flag. It'd be nice to instead let std/start.zig catch error.StdCliHelp and error.StdCliUsage and gracefully print help text there. But that's not currently possible, since std/start.zig has no idea what the shape of your Args is -- this would need more integration with the juicy main proposal ([https://github.com/ziglang/zig/issues/24510](https://github.com/ziglang/zig/issues/24510))

agreed, but i think a particularly nice feature of not including this in juicy main is control over the allocator that's used for the allocations. the user might not want to use `_start`'s default allocator, or may want to have all the allocations done through a nested arena...

### pancelor

2026-01-06T23:25:27+01:00

the `@"--": void` thing isn't related to the arbitrary-positionals problem, you'd still need something like my `.positional.extra_files`: (with `@"--": void` it'd just be `.extra_files`)

```zig
        // a single list-of-strings field is allowed.
        // if present, extra positional args are allowed and collected here. must be the last field.
        // note: this is orthogonal to handling `--` properly.
        //   for instance, `docgen -- foo` would set `.input` to `"foo"`, and `extra_files` would be empty.
        extra_files: []const []const u8,
```

I agree that the juicy main stuff is off the table for now, I was just mentioning it as something I thought through and then realized it couldn't be done without something juicy-main-like.

> i still disagree that program\_name is better than arg0

Ah yes, I reread your reasoning and it makes a lot of sense. I have more thoughts but I'll follow your lead and leave the bikeshedding for PR comments, that seems wise

### dotcarmen

2026-01-06T23:59:21+01:00

> the @"--": void thing isn't related to the arbitrary-positionals problem, you'd still need something like my .positional.extra\_files: (with @"--": void it'd just be .extra\_files)

ah - the original PR already allows arbitrary positionals including without the `--` on the command line - so in your case, `docgen foo` and `docgen -- foo` would parse the same. the difference is that `--` (in the original PR and in my incoming PR) allows for escaping flag parsing. As an example:

```zig
const Args = struct {
    foo: bool = false,
    @"--": void = {},
    extra_files: []const []const u8,
};

pub fn main(init: Init) !void {
    const stderr = std.debug.lockStderr().terminal().writer;
    defer std.debug.unlockStderr();
    const args = try cli.parse(Args, init.allocator, init.minimal.args, .{ .writer = writer, .exit = true });

    try stderr.print(
        \\foo: {}
        \\extra files: 
    , .{ args.foo });
    for (args.extra_files) |f| try stderr.print("'{s}'", .{});
    try stderr.writeByte('\n');
}
```

```text
$ myprog --foo foo
foo: true
extra files: 'foo'
$ myprog -- --foo foo
foo: false
extra files: '--foo' 'foo'
$ myprog --foo -- foo
foo: true
extra files: 'foo'
```

### pancelor

2026-01-07T00:12:07+01:00

I'm not convinced about the `@"--": void` flattening. It does seem a little nice to be able to refer to `my_args.input` rather than `my_args.positional.input` but it adds another bit of magic and incomprehensibility for newcomers. Plus it's slightly different from the usage on the commandline, which might be confusing? Let me spell out an example:

```zig
const Args = {
  port: usize = 6000,

  @"--": void,
  input: []const u8,
  output: []const u8,
  third_thing: []const u8,
};
const args = ...parse(Args, ...);
```

When invoked as `myprog in.txt -- --port 8080`, the result is the same as:

```zig
const args = Args{
  .input = "in.txt",
  .output = "--port",
  .third_thing = "8080",
};
```

That's correct, that's exactly what we want `--` to do, but maybe it feels a bit confusing that the `Args` definition goes `--, input, output` but the order in the invocation is `input, --, output`.

...actually, now that I've written that all out, I'm more convinced, that seems fine. But is the extra magic worth it? `.named` / `.positional` seems lots more understandable to newcomers, and they'll probably run into this pretty early on.

Idk how to balance the goals here (easy to implement and maintain / friendly for new users / useful for power users / ?) so I'm leaning towards sticking to the original proposal for now. I dunno really, I'm on the fence about it.

### pancelor

2026-01-07T00:34:31+01:00

Another design thing I'm running into again: there's an impossible tug-of-war of properties we want to satisfy:

*   A. we want to allow mixing the order of named and positional args
*   B. we want boolean named args to be special, indicating truthyness with just `--foo` and not requiring `--foo true` or `--foo=true`
*   C. `--path ./my/file.txt` is nicer for filepath tab-completion (and familiarity), compared to `--path=./my/file.txt`
*   D. we wish `--foo bar` was unambiguous somehow. As-is, is this a named boolean `foo` and unrelated positional `bar`, or is it a named `foo` with value `bar`?
    *   we could require named non-bool args to use `=`, but that breaks C
    *   we could require named bool args to have a value like other named args, but that breaks B

The original proposal satisfies A B C but not D, and has some good reasoning:

> *   Either a space or an equal sign can separate the name and the value, e.g. `--name value` or `--name=value`. (Any literal `=` in a field name, e.g. `@"conf=usion"` would cause a compile error.) Motivation for space separation: shell tab-completion works best on space-separated tokens, e.g. for file paths. Motivation for equals-separated: when constructing an `args` array to launch a child process, a single append call possibly including string concatenation is simpler than two append calls or an extend call with two items. Remember that this is a consideration for all programming languages that might call a Zig executable, not just relevant from within Zig code. Additional motivation for equals: the relationship between names and values is self-documenting, e.g. `--a=b c d` is more self-documenting than `--a b c d`.

I think the solution in the proposal is good enough, if a little upsetting.

* * *

On second thought, property A is unrelated; `myprog --a --b --c foo bar` has the same problem. It's unclear to me whether property A was intended in the original proposal. (But based on a skim of the PR, it looks like it was intended)

### dotcarmen

2026-01-07T01:58:02+01:00

See [#30725](/ziglang/zig/issues/30725) for the new implementation PR :)

### JanBeelte

2026-01-07T17:12:54+01:00

I am currently trying to port dump-cov.zig and would need a single optional positional argument, would this still fall under the `extra_files: []const []const u8` syntax or should this be handled differently? I intuitively tried:

```text
struct {
       named: struct {},
       positional: struct {
           exe_file: [:0]const u8,
           cov_file: [:0]const u8,
           target: ?[:0]const u8,
       },
}
```

which does not seem to be supported currently, does this fall under the "non minimal" category already?

[@dotcarmen](/dotcarmen) latest state is here: [https://codeberg.org/JanBeelte/zig/src/branch/dotcarmen-cli](https://codeberg.org/JanBeelte/zig/src/branch/dotcarmen-cli)

### NicoElbers

2026-01-08T01:24:51+01:00

Incidentally I also started work on this yesterday, but hadn't check back in on the issue so far. I'll throw in some of my own thoughts.

[@JanBeelte](/JanBeelte) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9626198):

> I am currently trying to port dump-cov.zig and would need a single optional positional argument, would this still fall under the `extra_files: []const []const u8` syntax or should this be handled differently? I intuitively tried:
> 
> ```text
> struct {
>        named: struct {},
>        positional: struct {
>            exe_file: [:0]const u8,
>            cov_file: [:0]const u8,
>            target: ?[:0]const u8,
>        },
> }
> ```
> 
> which does not seem to be supported currently, does this fall under the "non minimal" category already?
> 
> [@dotcarmen](/dotcarmen) latest state is here: [https://codeberg.org/JanBeelte/zig/src/branch/dotcarmen-cli](https://codeberg.org/JanBeelte/zig/src/branch/dotcarmen-cli)

What I currently have is that I allow the parser to accept `?T` however only parse it as `T`. So the default value may be null, however if the argument is ever present it is guaranteed not null. I haven't looked at the PR, but in my code this was fairly simple to implement.

This is a nice property to have on positional arguments, however it may be detrimental on named options because it means you can't 'undo' the option. (eg. if you have `colour: ?bool = null`, after `--colour` you cannot get back to null, which might be nice if your program is aliased to provide that option)

[@pancelor](/pancelor) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9608531):

> I'm not convinced about the `@"--": void` flattening. It does seem a little nice to be able to refer to `my_args.input` rather than `my_args.positional.input` but it adds another bit of magic and incomprehensibility for newcomers. Plus it's slightly different from the usage on the commandline, which might be confusing? Let me spell out an example:
> 
> ```zig
> const Args = {
>   port: usize = 6000,
> 
>   @"--": void,
>   input: []const u8,
>   output: []const u8,
>   third_thing: []const u8,
> };
> const args = ...parse(Args, ...);
> ```
> 
> When invoked as `myprog in.txt -- --port 8080`, the result is the same as:
> 
> ```zig
> const args = Args{
>   .input = "in.txt",
>   .output = "--port",
>   .third_thing = "8080",
> };
> ```
> 
> That's correct, that's exactly what we want `--` to do, but maybe it feels a bit confusing that the `Args` definition goes `--, input, output` but the order in the invocation is `input, --, output`.
> 
> ...actually, now that I've written that all out, I'm more convinced, that seems fine. But is the extra magic worth it? `.named` / `.positional` seems lots more understandable to newcomers, and they'll probably run into this pretty early on.
> 
> Idk how to balance the goals here (easy to implement and maintain / friendly for new users / useful for power users / ?) so I'm leaning towards sticking to the original proposal for now. I dunno really, I'm on the fence about it.

I remain unconvinced of this approach. It feels like a layer of unnecessary complexity where `args.named` and `args.positional` really are not that big of a cost to pay. In my mind it makes much more sense that if I input `struct { foo: u8 = 0, bar: u8 = 1, baz: u8 = 2 }` that I'm able to find that struct back exactly, and not that the field name and position is semantically important to the result of my argument parser.

[@pancelor](/pancelor) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9609323):

> Another design thing I'm running into again: there's an impossible tug-of-war of properties we want to satisfy:
> 
> ```
> * A. we want to allow mixing the order of named and positional args
> 
> * B. we want boolean named args to be special, indicating truthyness with just `--foo` and not requiring `--foo true` or `--foo=true`
> 
> * C. `--path ./my/file.txt` is nicer for filepath tab-completion (and familiarity), compared to `--path=./my/file.txt`
> 
> * D. we wish `--foo bar` was unambiguous somehow. As-is, is this a named boolean `foo` and unrelated positional `bar`, or is it a named `foo` with value `bar`?
>   
>   * we could require named non-bool args to use `=`, but that breaks C
>   * we could require named bool args to have a value like other named args, but that breaks B
> ```
> 
> The original proposal satisfies A B C but not D, and has some good reasoning:
> 
> > *   Either a space or an equal sign can separate the name and the value, e.g. `--name value` or `--name=value`. (Any literal `=` in a field name, e.g. `@"conf=usion"` would cause a compile error.) Motivation for space separation: shell tab-completion works best on space-separated tokens, e.g. for file paths. Motivation for equals-separated: when constructing an `args` array to launch a child process, a single append call possibly including string concatenation is simpler than two append calls or an extend call with two items. Remember that this is a consideration for all programming languages that might call a Zig executable, not just relevant from within Zig code. Additional motivation for equals: the relationship between names and values is self-documenting, e.g. `--a=b c d` is more self-documenting than `--a b c d`.
> 
> I think the solution in the proposal is good enough, if a little upsetting.
> 
> On second thought, property A is unrelated; `myprog --a --b --c foo bar` has the same problem. It's unclear to me whether property A was intended in the original proposal. (But based on a skim of the PR, it looks like it was intended)

Is the simple solution to this problem not to simply disallow `--foo false` syntax? Something discussed in the original PR is having bool syntax be `--foo` to enable and `--no-foo` to disable. Then fields starting with `no-` would have to be disallowed, but that feels like a fine tradeoff.

### NicoElbers

2026-01-08T01:29:44+01:00

On another note I would consider adding colour, (and thus taking in an `Io.Terminal`). It's small but makes things look much nicer

### pancelor

2026-01-08T01:42:10+01:00

> Is the simple solution to this problem not to simply disallow --foo false syntax?

No, `myprog --foo bar` is still ambiguous if you don't know what type foo is -- could be bool, could be u32 or other nonbool, but the structure of the command doesn't tell you which.

Hopefully it won't matter very often because the arg names will be more descriptive than "foo", but it would be ideal if satisfying A B C and D simultaneously was possible.

### NicoElbers

2026-01-08T01:46:42+01:00

[@pancelor](/pancelor) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9634334):

> > Is the simple solution to this problem not to simply disallow --foo false syntax?
> 
> No, `myprog --foo bar` is still ambiguous if you don't know what type foo is -- could be bool, could be u32 or other nonbool, but the structure of the command doesn't tell you which.
> 
> Hopefully it won't matter very often because the arg names will be more descriptive than "foo", but it would be ideal if satisfying A B C and D simultaneously was possible.

That's a fair way of looking at it, I would argue though that that is what `--help` is for. If `--foo` is a common option, you're likely to know it would be a bool. If it is not a common option, you're unlikely to know about `--foo` at all unless you've seen the help message, which tells you the type.

### andrewrk

2026-01-08T11:15:38+01:00

I veto `@"--": void,` on the account of it being strange and overcomplicated. I don't recognize "use-site of CLIArgs polluted by named/positional distinction" as a valid concern. If anything it will get people used to using field access, and thereby ready to embrace composition over inheritance.

### dotcarmen

2026-01-08T15:43:40+01:00

i'll undo the `@"--"` change in the PR later today (i didn't find time to work on the PR yesterday, though [@JanBeelte](/JanBeelte) did some work that I can pull in)

* * *

[#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9634136)

> What I currently have is that I allow the parser to accept ?T however only parse it as T. So the default value may be null, however if the argument is ever present it is guaranteed not null. I haven't looked at the PR, but in my code this was fairly simple to implement.

Yeah, in the original issue and PR, optional positionals are allowed as long as there's no variadic positionals. optional named args aren't allowed. However it doesn't recognize `?T` which i'll have to add, though i think it's fair to add the restriction that the default value (if specified) must be `null` since that's what the parser will insert if the argument is missing

> This is a nice property to have on positional arguments, however it may be detrimental on named options because it means you can't 'undo' the option. (eg. if you have colour: ?bool = null, after --colour you cannot get back to null, which might be nice if your program is aliased to provide that option)

I also think it's confusing - let's say it's `foo: ?[:0]const u8 = null` - personally, i'd intuit this to indicate that a value doesn't have to be passed to `--foo`, even though that's not possible to correctly determine

But now you have the problem of diverging behavior for `named` and `positional`... there's already a (reasonable) difference for `bool` fields, but now you also have optional fields being allowed in one but not the other...

Having written that, I think it's fine to allow `?T` in both named and positional args, but to clearly document that named arguments will will always parse the succeeding value appropriately, and so the parser will *never* set the value to `null` unless the argument is missing and it's the default value

> Is the simple solution to this problem not to simply disallow --foo false syntax? Something discussed in the original PR is having bool syntax be --foo to enable and --no-foo to disable. Then fields starting with no- would have to be disallowed, but that feels like a fine tradeoff.

Yeah, this is the path that was followed at the end of the original PR, and I've kept this behavior in mine

> On another note I would consider adding colour, (and thus taking in an Io.Terminal). It's small but makes things look much nicer

Agreed, i'll update the PR to use this as well

* * *

[#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9634334)

> No, myprog --foo bar is still ambiguous if you don't know what type foo is -- could be bool, could be u32 or other nonbool, but the structure of the command doesn't tell you which.

while true, i'm not a fan of "attempt to parse value, and if it doesn't parse correctly then ignore it" because that may not be the user's intention (`--foo --bar` where `foo: ?[:0]const u8` is a better example for the ambiguity)

### NicoElbers

2026-01-08T16:17:09+01:00

[@dotcarmen](/dotcarmen) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9648032):

> But now you have the problem of diverging behavior for `named` and `positional`... there's already a (reasonable) difference for `bool` fields, but now you also have optional fields being allowed in one but not the other...

Nullable named and positional arguments were always going to have somewhat different behavior. For example semantically this makes no sense: `positional: struct { foo: ?[]const u8, bar: []const u8 }`. If foo is optional, and bar is required, what does that mean if you only provide one positional argument. Personally I think this should be disallowed, but you can make a reasonable argument that for the sake of simplicity we ignore this.

### NicoElbers

2026-01-08T16:22:14+01:00

Another, arguably simpler, approach that I believe the original issue also mentioned was to indeed just simply disallow nullable arguments completely. For the above mentioned `target: ?[:0]const u8` case, that we instead have `target: [:0]const u8 = "native"` for example. Having written out the previous message I'm leaning more and more towards this approach.

The original reason I added nullable named arguments to my parser was to accommodate something like `timeout_s: ?u32` as an argument, where having no value (null) has a real meaning. But I am starting to believe this may fall out of scope for this minimal parser.

### dotcarmen

2026-01-08T16:31:00+01:00

> For example semantically this makes no sense: positional: struct { foo: ?\[\]const u8, bar: \[\]const u8 }. If foo is optional, and bar is required, what does that mean if you only provide one positional argument.

i'll have to see what the code is right now, but i believe this should be a compile error - optional positionals can only come after required positionals.

### castholm

2026-01-09T00:36:20+01:00

[@pancelor](/pancelor) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9609323):

> Another design thing I'm running into again: there's an impossible tug-of-war of properties we want to satisfy:
> 
> *   A. we want to allow mixing the order of named and positional args
> 
> *   B. we want boolean named args to be special, indicating truthyness with just `--foo` and not requiring `--foo true` or `--foo=true`
> 
> *   C. `--path ./my/file.txt` is nicer for filepath tab-completion (and familiarity), compared to `--path=./my/file.txt`
> 
> *   D. we wish `--foo bar` was unambiguous somehow. As-is, is this a named boolean `foo` and unrelated positional `bar`, or is it a named `foo` with value `bar`?
> 
>     *   we could require named non-bool args to use `=`, but that breaks C
>     *   we could require named bool args to have a value like other named args, but that breaks B

If we assume somewhat "standard" command line conventions, it's important to understand that in isolation and without any sort of context, a command line like `--foo bar` will always be ambiguous. Unlike e.g. a URL query string, you can't tokenize a command line (and especially not provide user-friendly error messages) into positional arguments and options without the parser having knowledge of some sort of schema over available options and whether they take a required value, an optional value or no value at all. Without a schema, `--foo bar` could be parsed as "the option `--foo` with its value set to `bar`", or "the valueless option `--foo`, followed by a positional argument `bar`".

If we look at what most conventional CLIs do, if an option `--foo` is defined to take a required value, then

*   `--foo` followed by EOF is an error
*   `--foo bar` and `--foo=bar` are both parsed as the option `--foo` with its value set to `bar`

If `--foo` is defined to not take a value, then

*   `--foo` followed by EOF is parsed as the valueless option `--foo`
*   `--foo=bar` is an error
*   `--foo bar` is parsed as the valueless option `--foo`, followed by a positional argument `bar`

Options that take optional values can be assigned a value using `=`, and will fall back to a default value otherwise. In other words, if `--foo` is defined to take an optional value, then

*   `--foo` followed by EOF is parsed as the option `--foo` with its value set to some default value
*   `--foo=bar` is parsed as the option `--foo` with its value set to `bar`
*   `--foo bar` is parsed as the option `--foo` with its value set to some default value, followed by a positional argument `bar`

To back up my assertion that it's conventional to handle optional options this way, let's use `git tag` as an example:

```text
> git tag --help
...
Tag listing options
    --column[=<style>]    show tag list in columns
...

> git tag --list --column
1.0     1.2     1.4     1.6     1.8     2.1     column  row
1.1     1.3     1.5     1.7     2.0     bar     foo

> git tag --list --column=row
1.0     1.1     1.2     1.3     1.4     1.5     1.6     1.7     1.8
2.0     2.1     bar     column  foo     row

> git tag --list --column row
row
```

Zig itself also handles many (all?) of its optional options similarly, e.g. `--release[=mode]`.

### dotcarmen

2026-01-09T15:24:07+01:00

> Unlike e.g. a URL query string, you can't tokenize a command line (and especially not provide user-friendly error messages) into positional arguments and options without the parser having knowledge of some sort of schema over available options and whether they take a required value, an optional value or no value at all.

Luckily the basic requirement of this proposal is that there *is* a schema ;)

> Zig itself also handles many (all?) of its optional options similarly, e.g. --release\[=mode\].

many\*. Some optional values are the inverse - they *require* the value to be specified in the next arg ie `--zig-lib-dir [arg]`.

I think I've found a compromise that would allow bare named non-bool flags when the API consumer wants them... consider this:

```zig
const Args = struct {
    named: struct {
        foo: [:0]const u8 = "default",
        bar: ?[:0]const u8 = "default",
    },
};
```

The parser could then handle the possible scenarios:

*   `foo`:
    *   if `--foo` is absent, `named.foo` is `"default"`
    *   if `--foo=bar` or `--foo bar` is specified, `named.foo` is `"bar"`
*   `bar`:
    *   if `--bar` is absent, `named.bar` is `"default"`
    *   if `--bar` is specified, `named.bar` is `null`
    *   if `--bar=baz` is specified, `named.bar` is `"baz"`

It *does* add some complexity:

*   some named args would require `=`, while others would allow the value to be passed on a separate arg
*   what if `bar: ?[:0]const u8 = null`? should this require `=` or is it fine to allow the value to be passed in a separate arg?

Personally, I don't see the benefit in requiring `=`, and I'm not sure I see the benefit in the compromise I just suggested. Many modern CLIs (written in Rust, Go, JS) don't require `=` but allow it. Notably, Zig's cli has a mix of requiring `=` and requiring the value to be specified in the next arg (for example `--zig-lib-dir` and `--build-runner` *require* the value to be the next arg). I don't think there's a reasonable solution that satisfies all 4 properties listed in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9609323), and, admittedly selfishly, I believe shell path tab-completion is too important to sacrifice in the name of eliminating ambiguities that... i don't believe cause problems very often? idk...

### NicoElbers

2026-01-10T23:05:19+01:00

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9648734):

> Another, arguably simpler, approach that I believe the original issue also mentioned was to indeed just simply disallow nullable arguments completely. For the above mentioned `target: ?[:0]const u8` case, that we instead have `target: [:0]const u8 = "native"` for example. Having written out the previous message I'm leaning more and more towards this approach.
> 
> The original reason I added nullable named arguments to my parser was to accommodate something like `timeout_s: ?u32` as an argument, where having no value (null) has a real meaning. But I am starting to believe this may fall out of scope for this minimal parser.

I'm coming back on this, for two reasons:

1.  while working through upgrading some tools I've found that using the empty string as a sentinel is very annoying
2.  I think I have a solution for:

> you can't 'undo' the option. (eg. if you have `colour: ?bool = null`, after `--colour` you cannot get back to null, which might be nice if your program is aliased to provide that option)

Since we already have the precedent for booleans that `--no-foo` disables the option, the same can be done for nullable arguments. So we could have `./prog --colour=true --no-colour` which would result in `colour` being null.

I argue this does not add much complexity for users as options starting with `no-` are already disallowed, therefore the general rule becomes "any option starting with `no-` takes no value". It is also fairly simple to give a helpful error message when a user does `--no-foo` on a non nullable option.

### lukeflo

2026-01-13T10:48:23+01:00

Hi, I've just started to deal with Zig two weeks ago. My learning project is a very small and dep-free [CLI arg parser](https://codeberg.org/lukeflo/lexopts) (As said, I'm learning and the code might be very "bad" 😁).

While adopting my code to the new `std.process.Init` argument handling, I stumbled over this PR and the preceding one on Github. Its a very interesting discussion and I'm learning a lot from it in passing (e.g. handling of `@"string-field"`, thanks for that).

But what leaves me a bit irritated is the non-handling of short options. I know in the original PR the following is stated in the OP:

> Field names are prefixed by --, never single - and never /. This is true even if you name a field with a single letter, e.g. --n=100. Motivation: sometimes single - means that multiple single-letter options can be grouped together, like ls -lA, but double -- never has this ambiguity; although /-prefixed names are common on Windows, --prefixed names are also common, and CLI users will just need to deal with it.

Aside that there seems to be no real discussion of those cases. I grepped through the PR and didn't find anything.

I understand that parsing short opts adds complexity, especially if stacked short options like `ls -lah` should be taken into account too. And that including short options makes it difficult to extract option names from the `struct` field names directly.

However, short options are so convenient to use when working with the CLI that I think a simple CLI args implementation should at least discuss it taking them into account. Since the goal of these PRs is to offer a simple CLI parser which on one hand distinguishes between options, values (separated by space, `=` etc), named positional args and the rest of pos args in an elaborated way, ignoring short options seem to call the entire concept of such a simple parser into question.

Most people who write even only a small CLI tool will consider short options. But in such a (very common) case the built-in-to-be parser would be no possibility and they'd have to use an external library anyway (or write a custom handling of short options).

*This is in no way intended as criticism of your efforts*, because I welcome the concept of an integrated parsing functionality. I was just wondering because I couldn't find any discussion regarding this. It seems to be simply taken for granted following the original OP on Github.

Of course, it is possible that I have overlooked relevant considerations. In that case, just ignore this post completely ☺️.

In any case, thank you for the great work.

### NicoElbers

2026-01-13T11:36:46+01:00

Hai [@lukeflo](/lukeflo), welcome to Zig!

If I recall correctly the original issue did touch on the subject, and concluded it out of scope. If I understood correctly even going as far as to error on any option starting with a single '-'. The cli parser for this issue is meant to be quite minimal, with the potential of more advanced features down the line.

That being said, I think its fair to have some discussion on the topic as it is indeed a very common and nice feature to have.

There are 2 problems I see with the inclusion of short options:

1.  There is no obvious way, at least that I can think of, to configure them
2.  You lose the uniformity (and parsing simplicity) of having all non positional arguments start with '--'.

For point 1, the best idea I can think of is a simmilar approach as to how help is handled both in [#30725](/ziglang/zig/issues/30725) and my own WIP implementation (that being `pub const help = .{ .foo = "bar" };`). The immediate problem would then be what name to give that decl. `help` is already a reserved option so thats fine, but other name could we use?

For point 2, I am unsure how simple or complex this would be in reality. I would have to try, and I think I might.

Having written this down, thinking about it for 20 or so minutes and considering the complexity automatic help messages bring already, I would like to do my best to find a reasonable solution for short options.

If anyone has any ideas on hoe to tackle point 1 specifically (how to configure short optiona) I would love to hear your ideas!

### lukeflo

2026-01-13T12:33:25+01:00

Hey [@NicoElbers](/NicoElbers) , thanks for the fast reply.

One general question which came up to my mind is: what is considered to be still "minimal"/"simple" and when does it get "too complex"? And, is this simplicity meant to be only the user experience when using the parser for an external project, or is simplicity/non-complexity also a goal for the backend code in `std` which most users never see/touch? Thus, is, for instance, the simplicity of "having all non positional arguments start with `--`" considered simplicity in the user code or the `std` backend? Plus, which features are part of such a "minimal" setup, in general? E.g. *personally*, I think short options should be considered more important for a "minimal" user experience than automatically generated help messages.

Regarding my own library, processing short options in the backend is relatively simple and still works (unexpectedly) well (less than 100 lines of code). However, the user experience is significantly more complex as in the examples above, as an `enum`, a `staticStringMap`, and an (optional) `struct` must be created in order to correctly process the options returned by the parser. (tbh, there might be better ways, but I haven't found them so far... 😁)

I had a look at your `arg-parse` project too and it looks very interesting. But as I don't have much experience is Zig, its still hard for me to evaluate code parts regarding their complexity/performance etc. Thus, I can't say if any of the mentioned stuff/libs might help with one of you named points. But hopefully that'll change soon ;)

### NicoElbers

2026-01-13T13:18:56+01:00

Re: what is considered simplicity  
 I don't have a concrete definition of that either. My approach so far has been looking at the tradeoff between what the feature would add and how complex it is to implement and understand. For example having the help message aligned across all options was a surprisingly complex feature to implement (although I'm looking to simplify it), but not having it makes the experience so much worse that I am willing to put in the effort.

Specifically for "having all non positional arguments start with `--`". That's a nice property that makes my implementation a tiny bit simpler. Additionally, if I start allowing short options you get the issue that `-1` is a little more ambiguous. Is it a short option of `1` or the value `-1`. Not a massive problem, but another reason I hadn't really considered this until now.

Re: your library  
 I think your library actually gets a couple of things right that I did not. The use of a static string map and enum is quite a nice idea. It allows easy mapping of aliases to options (like short versions) and makes handing 'special' arguments (like `--help`) easier. Same with using an actual state machine for the iterator, no clue why I didn't do that before. I'm likely going to steal those.

### lukeflo

2026-01-13T13:37:30+01:00

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9788462):

> Re: your library  
>  I think your library actually gets a couple of things right that I did not. The use of a static string map and enum is quite a nice idea. It allows easy mapping of aliases to options (like short versions) and makes handing 'special' arguments (like `--help`) easier. Same with using an actual state machine for the iterator, no clue why I didn't do that before. I'm likely going to steal those.

Of course! I'm happy if anything I've written with my very limited Zig-knowledge helps other users

### Khitiara

2026-01-13T17:02:30+01:00

a somewhat absurd suggestion, but why not use something like augmenting types ala the `std.fmt.Alt` type for configuring options? the defaults can stay the defaults, and then have a type like (as a rough draft)

```zig
pub const ArgOptions = struct {
    aliases: []const []const u8,
    description: []const u8
};
pub inline fn WithOptions(comptime T: type, comptime opts: ArgOptions) type {
    return struct {
        pub const options = opts;
        value: T,
    };
}
```

the type's options decl can be accessed from the field type pretty easily and if it matches the shape (maybe renaming to avoid chances of collisions here) the options are available to use and if not just use the default behavior already described

could put short aliases in there too, for short opts. imo its reasonable that a short opt should never exist without a long opt version anyway

### dotcarmen

2026-01-13T23:34:42+01:00

welcome to Zig [@lukeflo](/lukeflo) :)

regarding short flag args, I agree - it would be a large benefit. However, since it was declared out-of-scope in the initial discussions, I figured it could be discussed later after the initial implementation PR lands.

[@Khitiara](/Khitiara) I think that solution is unnecessarily complicated. A possible extension of the current design would be:

```zig
pub const Args = struct {
    named: struct {
        foo: []const u8,
        pub const help = .{
            .foo = "does a foo thing",
        };
        pub const short = .{
            .foo = 'f',
        };
    },
};
```

Again, though, discussions around short flags won't impact my PR

Speaking of which, I had a very busy weekend and couldn't find time to continue the PR. However, I hope to have the PR ready to review in the next couple of days (i'm finishing up implementation details right now, and will be merging [@JanBeelte](/JanBeelte)'s commits and adding tests tomorrow)

### NicoElbers

2026-01-14T12:13:04+01:00

[@dotcarmen](/dotcarmen) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9815117):

> regarding short flag args, I agree - it would be a large benefit. However, since it was declared out-of-scope in the initial discussions, I figured it could be discussed later after the initial implementation PR lands.

I just checked the original issue, and I cannot find anywhere where this was actually discussed. The issue declares this out of scope in its initial version and it's never talked about. I think this is a reasonable argument to start that discussion here, given the how common and useful short options are.

> ```zig
> pub const Args = struct {
>     named: struct {
>         foo: []const u8,
>         pub const help = .{
>             .foo = "does a foo thing",
>         };
>         pub const short = .{
>             .foo = 'f',
>         };
>     },
> };
> ```

The problem with this design is that you can no longer use `short` (or any other name you'd use for the decl) as an option. I guess you can reasonably reserve a name for this and just have that be the way things are, but personally I would like to see if there's a better solution.

### lukeflo

2026-01-14T12:58:33+01:00

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9828512):

> I just checked the original issue, and I cannot find anywhere where this was actually discussed. The issue declares this out of scope in its initial version and it's never talked about.

Yes, that was also my impression as mentioned above and my reason for entering the discussion here. The author of the original issue, who is not working on this anymore if I got it right, just declared it out of scope without any detailed explanation. Maybe it didn't fit to his view of "simplicity" which again is not well defined either.

However, if a working PR without short options is accepted and is only later expanded, that's fine. But for me it seems more reasonable to find a simple but nevertheless as complete as possible solution directly. As mentioned, "simple" is the aspect in question here 😉

### Khitiara

2026-01-14T18:49:57+01:00

if a solution for short args can be found then the sooner the better just to avoid issues with back-compat once the initial version gets in. if not, then can always integrate in later

### dotcarmen

2026-01-14T23:56:21+01:00

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9724400):

> Since we already have the precedent for booleans that `--no-foo` disables the option, the same can be done for nullable arguments. So we could have `./prog --colour=true --no-colour` which would result in `colour` being null.
> 
> I argue this does not add much complexity for users as options starting with `no-` are already disallowed, therefore the general rule becomes "any option starting with `no-` takes no value". It is also fairly simple to give a helpful error message when a user does `--no-foo` on a non nullable option.

Sorry, I missed this comment 😅

I rather like this, actually. i'm pushing to a commit that passes CI tonight, but I think I'll add this to my PR in a follow-up commit.

* * *

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9828512):

> I just checked the original issue, and I cannot find anywhere where this was actually discussed. The issue declares this out of scope in its initial version and it's never talked about. I think this is a reasonable argument to start that discussion here, given the how common and useful short options are.

you're right... i thought there was pushback somewhere and i could look at the discussion later. apologies - i think i will give a stab at short flags in my PR, in that case. it's felt rather silly not to have them.

> The problem with this design is that you can no longer use `short` (or any other name you'd use for the decl) as an option. I guess you can reasonably reserve a name for this and just have that be the way things are, but personally I would like to see if there's a better solution.

huh, i forgot that's a name conflict.

[@Khitiara](/Khitiara)'s solution may not be all bad - internally, my parser uses enums to track the field type anyways (the original PR doesn't, and type handling is a lot less consistent), and i'm realizing it's not actually much more work to support something like that. However, i do think the extra redirect with `options` is unnecessary. i'd prefer:

```zig
pub const Args = struct {
    named: struct {
        foo: struct {
            value: []const u8,
            pub const description = "does a foo thing";
            pub const short = 'f';
        },
    },
};
```

### Khitiara

2026-01-15T00:11:31+01:00

that would also work, my only consideration with using an extra redirect with options is to allow using a function to make the type without losing any backwards compatibility.  
 i do think for the same sort of reason with the name clashes it might be best to have some sort of marker to suggest that the type is an arg with extra metadata rather than a type that happens to match the shape, though that could always be avoided with a simple wrapper when the type happens to match the shape

### dotcarmen

2026-01-15T01:11:14+01:00

> i do think for the same sort of reason with the name clashes it might be best to have some sort of marker to suggest that the type is an arg with extra metadata rather than a type that happens to match the shape, though that could always be avoided with a simple wrapper when the type happens to match the shape

so, maybe this is a hot take... i think it's fair to assume that the parser will *never* parse a struct value. `std.Build.option` doesn't allow struct options, and i think flattening struct field types would be confusing (ie `Args.named.foo.bar` would result in `--bar` or `--foo.bar` or `--foo-bar` or...)

### NicoElbers

2026-01-18T16:21:16+01:00

[@dotcarmen](/dotcarmen) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9847928):

> [@Khitiara](/Khitiara)'s solution may not be all bad - internally, my parser uses enums to track the field type anyways (the original PR doesn't, and type handling is a lot less consistent), and i'm realizing it's not actually much more work to support something like that. However, i do think the extra redirect with `options` is unnecessary. i'd prefer:
> 
> ```zig
> pub const Args = struct {
>     named: struct {
>         foo: struct {
>             value: []const u8,
>             pub const description = "does a foo thing";
>             pub const short = 'f';
>         },
>     },
> };
> ```

I personally dislike this idea because you lose the nice property of returning `Args` directly. In fact I would say it's actively harmful, if the eventual idea is to be able to write:

```zig
pub fn main (init: std.process.Init, args: struct {
    named: struct {},
    positional: struct {},
}) !void { ... }
```

which 

[lib/std/process.zig](https://codeberg.org/ziglang/zig/src/commit/114ea92c09b6f27fe7596fddc4b114a31bf1c334/lib/std/process.zig#L34-L36)

Lines 34 to 36 in [114ea92](https://codeberg.org/ziglang/zig/src/commit/114ea92c09b6f27fe7596fddc4b114a31bf1c334)

|  | `/// Completion of https://github.com/ziglang/zig/issues/24510 will also allow` |
|  | `/// the second parameter of the main function to be a custom struct that`      |
|  | `/// contain auto-parsed CLI arguments.`                                        |

seems to imply. You *need* to return `Args` directly.  
 Edit: On second thought you could of course say `args.named.foo.value` is the syntax to get the value, and that might be acceptable. However I would not say it's preferable.

Currently where my mind is at is to decouple these things a little more. The fields in `help` are already separated a little from the actual argument fields so why not just do this:

```zig
_ = try parse(init, .{}, struct {
    pub const short_aliases = .{
        .foo = 'f',
    };
    named: struct {
        foo: u8  = 0,
        
        pub const help = .{
            .foo = "foo help",
        };
    },

    positional: struct {
        bar: []const u8,
        
        pub const help = .{
            .bar = "bar help",
        },
    },
});
```

The distance between the declaration and 'options' is a bit unfortunate. But this is the best idea I have so far that maintains the properties:

*   Do not reserve any potential option names
*   Keep the api `fn parse(comptime T: type, ...) !T`
*   Allow for short aliases

### dotcarmen

2026-01-19T01:41:52+01:00

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9955716):

> I personally dislike this idea because you lose the nice property of returning `Args` directly.

Why wouldn't you be able to return `Args` directly? The proposed change would result in something like:

```zig
pub const Args = struct {
    named: struct {
        foo: struct {
            value: []const u8,
            pub const description = "does a foo thing";
            pub const short = 'f';
        },
    },
};

const args = try std.cli.parse(Args, ...);
std.debug.print("you entered: --foo={s}", .{ args.named.foo.value });
```

it's just one extra field access. no need to construct a new type that eliminates the `.value`. That still satisfies all the properties you listed while adding the nice property of keeping things locally-defined

* * *

edit: i missed your edit

> On second thought you could of course say `args.named.foo.value` is the syntax to get the value, and that might be acceptable. However I would not say it's preferable.

IMO it's better than having `Args.short_aliases` which is more action-at-a-distance...

### NicoElbers

2026-01-19T02:39:45+01:00

I initially glossed over this idea, but considering it now I find it hard to argue against.

To get a feeling myself, a more fully fledged usage would then look like this:

```zig
pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const args = try parse(init, struct {
        named: struct {
            foo: struct { value: u8 = 0 },
            bar: struct {
                value: enum { a, b, c } = .a,
                pub const description =
                    \\The important bar option
                    \\Represeting a, b or c
                ;
                pub const short = 'b';
            },
            qux: struct {
                value: bool = true,
                pub const short = 'q';
            },
        },

        positional: struct {
            input: struct { value: []const u8 = "./foo" },
            ouput: struct {
                value: []const u8,
                pub const description = "Some output thingy";
            },
        },
    });

    const file = try Io.Dir.cwd().openFile(io, args.positional.input.value, .{});
    defer file.close(io);
}
```

I think this is fine in terms of usage complexity as the entire structure of the `Args` struct has to be explained anyways (with `named` and `positional`).

Outside of that, as you mentioned, the integration of options/ arguments with their description, short version, or any other potential future configuration is more cohesive.

I think, assuming this is the approach we take, there are 2 minor open questions:

1.  Is `value` the correct name
2.  Do we disallow `foo: u8 = 0` in favor of `foo: struct { value: u8 = 0 }`

I think for (1), that `value` is a fine name to pick but perhaps someone has a better idea.

For (2) I think `foo: u8 = 0` *should* be disallowed. It means there's more uniformity in the definition, and any option will be able to 'get options' (like a description) without having to update any usage sites from `args.named.foo` to `args.named.foo.value`.

### yavko

2026-01-19T05:10:39+01:00

I implemented a similar idea in this library a while back [https://github.com/Seirea/dsargs](https://github.com/Seirea/dsargs), it wouldn't be too hard to extend it to add more features.

### dotcarmen

2026-01-19T13:46:38+01:00

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9965367):

> 2.  Do we disallow `foo: u8 = 0` in favor of `foo: struct { value: u8 = 0 }`
> 
> For (2) I think `foo: u8 = 0` *should* be disallowed. It means there's more uniformity in the definition, and any option will be able to 'get options' (like a description) without having to update any usage sites from `args.named.foo` to `args.named.foo.value`.

my initial reaction is "i think it's fine to allow it", but I think the uniformity you mentioned matches these points in Zig Zen

> *   Favor reading code over writing code.
> *   Only one obvious way to do things.
> *   Reduce the amount one must remember.

* * *

[@yavko](/yavko) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9966876):

> I implemented a similar idea in this library a while back [https://github.com/Seirea/dsargs](https://github.com/Seirea/dsargs), it wouldn't be too hard to extend it to add more features.

neat! have you seen [my PR](https://codeberg.org/ziglang/zig/pulls/30725)? it adds quite a few more features which is why the file ends up being quite a bit larger:

*   positional arguments
*   multiple forms of arg iteration (`std.process.Args`, `[]const []const u8`, custom iterator)
*   printing to any `std.Io.Terminal` instead of always printing to stderr (defaulting to stderr though)
*   help text for arguments (and alignment for consistency)
*   comptime-generated help and usage fmt strings
*   default argument values
*   if an argument fails to parse or a required argument is missing, optionally prints the error followed by usage, and optionally exits the program (exit code 0 for `--help`, exit code 1 for usage errors)
*   enum arguments
*   list-of int/float/enum/string arguments
*   strings may also be `[:0]const u8`

also, your library has leaks (you iterate args with the now-non-existent `std.process.argsWithAllocator`, but don't free the arg strings from the iterator, and dupe the strings for string argument fields). note that my PR suggests using an `ArenaAllocator` - I might end up changing this, but it's better to set users' expectations on memory management :)

### dotcarmen

2026-01-19T15:43:13+01:00

so i've implemented the `struct { value: <type> }` in [this commit](https://codeberg.org/dotcarmen/zig/commit/da00d6c5793cca66e5f1c402b27189f80f4275bb), however, a question that came up while i was doing this: should the default value be *inside* the struct, or should it be *of* the struct? for example:

```zig
const Args = struct {
    named: struct {
        // should it be this:
        foo: struct { value: bool = false },
        // or this:
        foo: struct { value: bool } = .{ .value = false },
    },
};
```

in the linked commit i did the first, but wanted to gather feedback ITT

### InKryption

2026-01-19T16:51:54+01:00

If I might throw my hat into the bikeshedding ring: I think it would be nice if help message was constructed from a separate, typed struct based off of the options, in order to both have nice type errors when you're missing any fields, and to have all of the descriptions consolidated close together, giving us a cohesive view of all of the help texts in a central part of the code:

```zig
pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const args = try parse(init, struct {
        named: struct {
            foo: struct { value: u8 = 0 },
            bar: struct {
                value: enum { a, b, c } = .a,
                pub const short = 'b';
            },
            qux: struct {
                value: bool = true,
                pub const short = 'q';
            },

            pub const description: std.cli.Description(@This()) = .{
                .foo = "",
                .bar = (
                    \\The important bar option
                    \\Representing a, b or c
                ),
                .qux = "",
            };
        },
        positional: struct {
            input: struct { value: []const u8 = "./foo" },
            ouput: struct { value: []const u8 },

            pub const description: std.cli.Description(@This()) = .{
                .input = "",
                .output = "Some output thingy",
            };
        },
    });
}
```

Would probably dovetail nicely with a design that combined the `named` and `positional` structs into one, i.e. the `@"--"` marker field solution:

```zig
const Args = struct {
    foo: struct { value: u8 = 0 },
    bar: struct {
        value: enum { a, b, c } = .a,
        pub const short = 'b';
    },
    qux: struct {
        value: bool = true,
        pub const short = 'q';
    },
    @"--": void = {},
    input: struct { value: []const u8 = "./foo" },
    ouput: struct { value: []const u8 },

    const description: std.cli.Description(@This()) = .{
        .foo = "",
        .bar = (
            \\The important bar option
            \\Representing a, b or c
        ),
        .qux = "",
        .@"--" = {},
        .input = "",
        .output = "Some output thingy",
    };
};

pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const args = try parse(init, Args, Args.description);
}
```

(I would additionally opine that declaring the `Args` struct beyond the scope of the argument is nicer as self-documenting code, and permits for the riddance of the magical `description` decl, avoiding any originally mentioned problems of reserved identifiers).

You could also perhaps move the magical `short` declarations into this descriptor struct, turning the whole `Args` struct back into just a normal struct, besides the potential `@"--"` marker field:

```zig
const Args = struct {
    foo: u8 = 0,
    bar: enum { a, b, c } = .a,
    qux: : bool = true,
    @"--": void = {},
    input: []const u8 = "./foo",
    ouput: []const u8,

    const description: std.cli.Description(@This()) = .{
        .foo = .{ null, "" },
        .bar = .{
            'b',
            \\The important bar option
            \\Representing a, b or c
        },
        .qux = .{ 'q', "" },
        .@"--" = {},
        // all fields after the marker are just strings, instead of `struct { ?u8, []const u8 }`
        .input = "",
        .output = "Some output thingy",
    };
};

pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const args = try parse(init, Args, Args.description);
}
```

### dotcarmen

2026-01-19T17:27:17+01:00

[@InKryption](/InKryption) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9988509):

> If I might throw my hat into the bikeshedding ring: I think it would be nice if help message was constructed from a separate, typed struct based off of the options, in order to both have nice type errors when you're missing any fields, and to have all of the descriptions consolidated close together, giving us a cohesive view of all of the help texts in a central part of the code:

I think type errors for missing fields are bad, since i think it's totally 100% valid to not want to provide documentation for options (internal tooling, prototyping, etc), but it's certainly nice that it guards against typos.

> Would probably dovetail nicely with a design that combined the `named` and `positional` structs into one, i.e. the `@"--"` marker field solution:

Andrew [already vetoed any designs attempting that](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9641471), so i'm gonna ignore your usage of that design (i should update my original comment to reflect the veto...)

I suppose this wouldn't be too bad, though:

```zig
const Args = struct {
    named: struct {
        foo: u8 = 0,
        bar: enum { a, b, c } = .a,
        quux: bool = true,
    },
    positional: struct {
        output: []const u8, // note: in your example, you had output first, but it's heavily complicated to support optional positionals before required ones
        input: []const u8 = "./foo",
    },

    pub const meta: std.cli.ArgMeta(@This()) = .{
        .arg0 = "myprog", // ?[]const u8 = null
        .description = "this program does a thing", // ?[]const u8 = null
        .named = .{
            .foo = .{ .short = 'f', .description = "foo thing" }, // struct { short: ?u8 = null, description: ?[]const u8 = null } = .{}
        },
        .positional = .{
            .output = .{ .description = "Some output thingy" }, // struct { description: ?[]const u8 = null } = .{}
        },
    };
};
```

> (I would additionally opine that declaring the `Args` struct beyond the scope of the argument is nicer as self-documenting code, and permits for the riddance of the magical `description` decl, avoiding any originally mentioned problems of reserved identifiers).

since we're not doing the flattening, and `Args` already has reserved identifiers, i think it's fine in this scenario to use `meta`, `options`, `descriptor`, or something else - though `description` in this context makes me think of a string...

### dotcarmen

2026-01-19T18:38:40+01:00

honestly, i'm a bit torn between the last 2 design suggestions for handling argument metadata.

```zig
// solution 1 - locality
const Args = struct {
    pub const description = "my program thingy";
    named: struct {
        foo: struct {
            value: bool = false,
            pub const description = "this is a description";
        },
    },
};

// solution 2 - intention
const Args = struct {
    named: struct {
        foo: bool = false,
    },

    // i said "meta" above, but i think "info" is cleaner
    pub const info: std.cli.ArgInfo(@This()) = .{
        .description = "my program thingy",
        .named = .{
            .foo = .{ .description = "foo thing" },
        },
    };
};
```

The biggest advantage solution 1 has is locality - everything is immediately apparent by surrounding context.

The biggest advantage solution 2 has is intention - everything you see is guaranteed to have meaning, with the only "unsafe" declaration being `meta` (it *could* be an argument to `parse`, but i think it's better to preserve *some* sense of locality)

I see a tie in Zen principles too - solution 1 is preferred by:

> *   Communicate intent precisely.
> *   Favor reading code over writing code.

Meanwhile, i think solution 2 is preferred by:

> *   Compile errors are better than runtime crashes. bugs.
> *   Reduce the amount one must remember.

Solution 1 *can* still have compile errors to detect unintended declarations (it already does for fields) but i feel like that's approaching a local maximum...

### NicoElbers

2026-01-19T19:25:42+01:00

[@dotcarmen](/dotcarmen) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9987318):

> so i've implemented the `struct { value: <type> }` in [this commit](https://codeberg.org/ziglang/zig/pulls/30725/commits/da00d6c5793cca66e5f1c402b27189f80f4275bb), however, a question that came up while i was doing this: should the default value be *inside* the struct, or should it be *of* the struct? for example:
> 
> ```zig
> const Args = struct {
>     named: struct {
>         // should it be this:
>         foo: struct { value: bool = false },
>         // or this:
>         foo: struct { value: bool } = .{ .value = false },
>     },
> };
> ```
> 
> in the linked commit i did the first, but wanted to gather feedback ITT

I would personally opt for the first, the second feels like an unnecessary amount of extra typing for no benefit that I can see.

### Khitiara

2026-01-19T19:36:59+01:00

[@dotcarmen](/dotcarmen) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9990267):

> The biggest advantage solution 1 has is locality - everything is immediately apparent by surrounding context.
> 
> The biggest advantage solution 2 has is intention - everything you see is guaranteed to have meaning, with the only "unsafe" declaration being `meta` (it *could* be an argument to `parse`, but i think it's better to preserve *some* sense of locality)
> 
> I see a tie in Zen principles too - solution 1 is preferred by:
> 
> > *   Communicate intent precisely.
> > *   Favor reading code over writing code.
> 
> Meanwhile, i think solution 2 is preferred by:
> 
> > *   Compile errors are better than runtime crashes. bugs.
> > *   Reduce the amount one must remember.
> 
> Solution 1 *can* still have compile errors to detect unintended declarations (it already does for fields) but i feel like that's approaching a local maximum...

I definitely lean more towards maintaining locality here as much as possible.

[@NicoElbers](/NicoElbers) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9991392):

> [@dotcarmen](/dotcarmen) wrote in [#30677](/ziglang/zig/issues/30677) (comment):
> 
> > so i've implemented the `struct { value: <type> }` in [this commit](https://codeberg.org/ziglang/zig/pulls/30725/commits/da00d6c5793cca66e5f1c402b27189f80f4275bb), however, a question that came up while i was doing this: should the default value be *inside* the struct, or should it be *of* the struct? for example:
> > 
> > ```zig
> > const Args = struct {
> >     named: struct {
> >         // should it be this:
> >         foo: struct { value: bool = false },
> >         // or this:
> >         foo: struct { value: bool } = .{ .value = false },
> >     },
> > };
> > ```
> > 
> > in the linked commit i did the first, but wanted to gather feedback ITT
> 
> I would personally opt for the first, the second feels like an unnecessary amount of extra typing for no benefit that I can see.

I would also opt more for the first for the same reason

### NicoElbers

2026-01-19T20:12:10+01:00

[@dotcarmen](/dotcarmen) wrote in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9990267):

> honestly, i'm a bit torn between the last 2 design suggestions for handling argument metadata.

I think solution 1 more readable. If you see option `foo`, you immediately see that it does (or doesn't) have a short alias, and what the option is about.

I think with this design you can avoid having compile errors when no description is required, because it's all very close together (It's obvious that you're missing the description decl).

Solution 2 has 2 nice properties in my opinion:

1.  It's an obvious place to configure program name (`arg0` in your examples), description, epilogue etc.
2.  You lose the arbitrary decls you have to remember.

I think you can combine the two though.

```zig
const Args = struct {
    pub const info: std.cli.Info = .{
        .name = "", // ?[]const u8 = null
        .description = "", // ?[]const u8 = null
        .epilogue = "", // ?[]const u8 = null
    };

    named = struct {
        foo: struct {
            value: bool = false,
            
            pub const info: std.cli.Named = .{
                .short = 'f', // ?u8 = null,
                .description = "", // ?[]const u8 = null,
            };
        },
        bar: struct { value: bool = false },
    },

    positional = struct {
        foo: struct {
            value: bool = false,
            
            pub const info: std.cli.Positional = .{
                .description = "", // ?[]const u8 = null,
            };
        },
        bar: struct { value: bool = false },
    },
};
```

This way you establish the `info` decl as the way to configure an option, which then is a concrete struct where you are presented with all available options. At the same time, you retain locality and there's no reason for you to need to throw away the 'top level' `info` decl.

This is again a little more writing, but I don't think it's a massive deal. Worst case you copy paste the previous declaration.

I do fear this may be starting to get a little cluttered, but I'd need someone elses eyes to judge how readable this is or isn't.

### dotcarmen

2026-01-20T19:16:12+01:00

Ok, I rather like the idea of `pub const info` as [@NicoElbers](/NicoElbers) suggested, thank you and [@Khitiara](/Khitiara) for the discussion :)

* * *

I've implemented optionals as suggested in [#30677 (comment)](https://codeberg.org/ziglang/zig/issues/30677#issuecomment-9724400) in [this commit](https://codeberg.org/ziglang/zig/commit/e7b8dd1c948a76c0de8706b9b4d95452bd459e50). I made the decision that `?T` positionals are treated as optional, and they *must* declare their default to `null` if they specify a default - it just doesn't make sense otherwise. Likewise, `?[]...` doesn't make sense, so it's not supported.

However, I'm wondering if named arguments with type `?T` that *don't* declare a default should also be treated as optional, and default to `null` if not provided. That is:

```zig
pub const Args = struct {
    positional: struct {
        foo: struct { value: ?u8 = null }, // standard optional form
        bar: struct { value: ?u8 }, // also optional
        baz: struct { value: u8 = 0 }, // also optional
        quux: struct { value: []const []const u8 }, // also optional - note that `?` doesn't make sense here, so it's a compile error
    },
    named: struct {
        foo: struct { value: ?u8 = null }, // standard optional form
        bar: struct { value: u8 = 0 }, // also optional
        baz: struct { value: ?u8 }, // Q: should this be treated as a required argument, or an optional argument with default `null`???
    },
};
```

I think the most consistent answer amongst named arguments is "treat `--bar` as required", however "treat `--bar` as optional" is more consistent with positional arguments' handling of optionals

### dotcarmen

2026-01-23T15:10:41+01:00

[my PR](https://codeberg.org/ziglang/zig/pulls/30725) is now open for review :)

### Tomcat-42

2026-01-26T22:20:58+01:00

Overall, I think I'm ready to be attacked on [#30953](/ziglang/zig/issues/30953). It was straightforward to port stuff in `tools` and I think the API is nice:

```zig
const Args = struct {
    struct { help: ?void = null, version: ?void = null },
    []const []const u8,
};

pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const allocator = init.arena.allocator();

    const args = std.cli.parse(Args, try init.minimal.args.toSlice(allocator), allocator) catch
        usage();

    const named, const positionals = args;

    if (named.version != null) version();
    if (named.help != null) usage();
}
```

(More examples on the [tests](https://codeberg.org/ziglang/zig/src/commit/0f61bdca7aa2f5f1ebec54dd13d9b7d9fd7278cf/lib/std/cli.zig#L302))

What do you think?

### justusk

2026-02-03T19:04:16+01:00

I think it would be nice to be able to name the `value` field however you want. This wouldn't be ambiguous since the struct it's in can only ever have one field that's relevant for parsing anyway (having more would just have to be a compile error) and it would be an opportunity to make the generated help text better by providing the field name as a type instead of the actual field type.

Especially for strings, having the help text say that `[string]` is expected can be pretty ambiguous, a string can be pretty much anything. Instead the generated help text could just use the name of the struct field as the expected type. Of course what is expected could also be mentioned in the description text instead, but I think it would be nicer this way and help with keeping descriptions short and focused.

The only case I could think of where this would be awkward are bools, but they could just ignore the field name instead.

```zig
const Args = struct {
    named: struct {
        verbose: struct {
            field_name_ignored: bool = false,
            pub const info = {...};
        },
        exclude: struct {
            pattern: ?[:0]const u8 = null,
            pub const info = {...};
        },
        @"add-entry": struct {
            @"<key>=<value>": []const [:0]const u8 = &.{},
            pub const info = {...};
        },
    },
    positional: struct {
        input: struct {
            filepath: [:0] const u8,
            pub const info = {...};
        },
    },
};
```

```text
Arguments:
    input                       [filepath. required]

Options:
    --[no-]verbose              [default: no]
    --exclude=pattern           exclude entries matching pattern
    --add-entry=<key>=<value>   [multiple] add new entry to input
```

### dotcarmen

2026-02-03T21:00:37+01:00

[@justusk](/justusk) i like that idea, but i'm afraid of how it would affect readability on access - specifically referencing `add-entry` from your example:

```zig
for (args.named.@"add-entry".@"<key>=<value>") |_| {}
// vs
for (args.named.@"add-entry".value) |_| {}
```

I think a more palatable solution would be exposing `typename` on `NamedInfo` and `PositionalInfo`, which would do the same thing:

```zig
const Args = struct {
    named: struct {
        exclude: struct {
            value: ?[:0]const u8 = null,
            pub const info: std.cli.NamedInfo = .{
                .typename = "pattern",
            };
        },
        @"add-entry": struct {
            value: []const [:0]const u8 = &.{},
            pub const info: std.cli.NamedInfo = .{
                .typename = "<key>=<value>",
            };
        },
    },
    positional: struct {
        input: struct {
            value: [:0]const u8,
            pub const info: std.cli.PositionalInfo = .{
                .typename = "filepath",
            };
        },
    },
};
```

### justusk

2026-02-03T21:25:18+01:00

[@dotcarmen](/dotcarmen) Thanks for the response, I would also be happy with a `typename` field! However I'd argue that this change would *increase* readability because it forces the developer to actually name things for what they are. Thinking one step further:

```zig
for (args.named.@"add-entry".value) |@"what was 'value' supposed to be again? I forgot :("| { ... }
// vs
for (args.named.@"add-entry".@"<key>=<value>") |kv| {
    const sep_idx = std.mem.findScalar(u8, kv, '=').?;
    const key = kv[0..sep_idx];
    const value = kv[sep_idx + 1 ..];
    { ... }
}
```

Having to name the field `value` every time just feels like a lost opportunity to store information.

### jeffective

2026-02-17T09:20:04+01:00

This approach has no magic decls, and thus no collisions with potentially useful field names, however, it loses some locality. The locality could be improved by the user by simply using their own decls in the `Args` type though.

```zig
const std = @import("std");

const ArgumentOptions = struct {
    order: enum { named, positional } = .named,
    shorthand: []const u8 = "",
    help: []const u8 = "",
};

pub fn SubcommandOptions(comptime S: type) type {
    return struct {
        help: []const u8 = "",
        inner: ParseOptions(S),
    };
}

pub fn ParseOptions(comptime T: type) type {
    return switch (@typeInfo(T)) {
        .@"struct" => |info| {
            var field_types: [info.fields.len]type = undefined;
            inline for (&field_types, comptime std.meta.fieldNames(T)) |*field_type, field_name| {
                field_type.* = ParseOptions(@FieldType(T, field_name));
            }
            const field_attrs: [info.fields.len]std.builtin.Type.StructField.Attributes = @splat(.{});
            return @Struct(
                .auto,
                null,
                std.meta.fieldNames(T),
                &field_types,
                &field_attrs,
            );
        },
        .@"union" => |info| {
            var field_types: [info.fields.len]type = undefined;
            inline for (&field_types, comptime std.meta.fieldNames(T)) |*field_type, field_name| {
                field_type.* = SubcommandOptions(@FieldType(T, field_name));
            }
            const field_attrs: [info.fields.len]std.builtin.Type.StructField.Attributes = @splat(.{});
            return @Struct(
                .auto,
                null,
                std.meta.fieldNames(T),
                &field_types,
                &field_attrs,
            );
        },
        else => ArgumentOptions,
    };
}

test "ParseOptions.union.sanity" {
    // this can parse the following:
    // git --log_level warn clone http://example.com
    // git add hello.txt world.txt
    const Args = struct {
        log_level: enum { debug, warn, err },
        subcommand: union(enum) {
            clone: struct {
                url: []const u8,
            },
            add: struct {
                paths: []const []const u8,
            },
        },
    };

    const options: ParseOptions(Args) = .{
        .log_level = .{
            .order = .named,
            .help = "set the log level",
        },
        .subcommand = .{
            .clone = .{
                .help = "clone a repo",
                .inner = .{
                    .url = .{
                        .order = .positional,
                        .help = "url to clone, example: http://example.com",
                    },
                },
            },
            .add = .{
                .help = "stage files",
                .inner = .{
                    .paths = .{ .order = .positional, .help = "the paths to stage" },
                },
            },
        },
    };
    _ = options;
}

test "ParseOptions.struct.sanity" {
    const Args = struct {
        url: []const u8,
    };

    const options: ParseOptions(Args) = .{
        .url = .{
            .order = .named,
            .shorthand = "u",
            .help = "url to clone, example: http://example.com",
        },
    };
    _ = options;
}

pub fn parse(comptime T: type, argv: [][:0]const u8, allocator: std.mem.Allocator, options: ParseOptions(T)) T {
    _ = argv;
    _ = allocator;
    _ = options;

    // the rest of the owl here
}

```

### lukeflo

2026-03-21T21:04:10+01:00

Since I rewrote some parts of my own little args parsing library, I going to throw it into the mix too: [lexopts](https://codeberg.org/lukeflo/lexopts)

Its approach is a little different from many others presented here, as well as the from the points of the initial issue:

*   no automatic help generation
*   nothing needs to be declared ahead of time
*   less than 200 lines of code

It heavily inspired by the similar named Rust crate `lexopt`

Here's an example how to use it:

```zig
const std = @import("std");
const lexopts = @import("lexopts");
const short = lexopts.matchShort;
const long = lexopts.matchLong;

pub fn main(init: std.process.Init) !void {
    // In order to allocate memory we must construct an `Allocator` instance.
    const alloc = init.arena.allocator();

    const CliArgs = struct {
        foo: bool,
        bar: []const u8,
        baz: u32,
        pos_args: std.ArrayList([]const u8),

        fn defaultArgs() @This() {
            return @This(){
                .foo = false,
                .bar = "default",
                .baz = 0,
                .pos_args = std.ArrayList([]const u8).empty,
            };
        }
    };

    var cli_args = CliArgs.defaultArgs();
    defer cli_args.pos_args.clearAndFree(alloc);

    // Collect args from the CLI using `std.process.Init` struct
    const argv = try init.minimal.args.toSlice(alloc);
    var parser = lexopts.Parser.init(argv);
    defer alloc.free(argv);

    std.debug.print("Binary: '{s}'\n\n", .{parser.bin_name});

    while (try parser.next()) |arg| {
        switch (arg) {
            .Option => |opt| {
                if (short(opt, 'h') or long(opt, "help")) {
                    const help_text =
                        \\Usage: lexopts [Options] [Positional Args...]
                        \\
                        \\--help       print help text
                        \\--version    show version
                        \\--foo        set foo=true
                        \\--bar=[STR]  set value of bar to STRING
                        \\--baz=[INT]    set value of baz to INTEGER
                    ;
                    std.debug.print("{s}\n", .{help_text});
                    std.process.exit(0);
                } else if (short(opt, 'v') or long(opt, "version")) {
                    std.debug.print("--version was called\n", .{});
                } else if (short(opt, 'b') or long(opt, "bar")) {
                    cli_args.bar = try parser.value();
                } else if (short(opt, 'f') or long(opt, "foo")) {
                    cli_args.foo = true;
                } else if (long(opt, "baz")) {
                    cli_args.baz = try std.fmt.parseInt(u32, try parser.value(), 10);
                } else {
                    return parser.unknownOpt();
                }
            },
            .PosArg => |value| {
                try cli_args.pos_args.append(alloc, value);
            },
        }
    }

    std.debug.print("'--foo' value: {}\n", .{cli_args.foo});
    std.debug.print("'--bar' value: {s}\n", .{cli_args.bar});
    std.debug.print("'--baz' value: {}\n", .{cli_args.baz});
    for (cli_args.pos_args.items, 0..) |cur_arg, i| {
        std.debug.print("Positional arg {d}: {s}\n", .{ i + 1, cur_arg });
    }
}
```

Don't know what other users think of it. But I like the way it doesn't forces me to declare a complicated `Args` struct before even parsing the command line.

Its one of my very first Zig projects so don't expect idiomatic code. I also don't know if an approach like this would fit into `std` lib. However, just wanted to present it 🙂

### jeffective

2026-03-30T07:01:02+02:00

After thinking more about this issue I began to question the entire "type driven" premise and decided the best way to present my counter argument was to provide a fully implemented more traditional / compositional design in [#31620](/ziglang/zig/issues/31620).
