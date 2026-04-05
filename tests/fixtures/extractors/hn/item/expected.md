# Magic: The Gathering Is Turing Complete (2019)

## Comments

Related:

*Magic: The Gathering is Turing Complete (2019)* - [https://news.ycombinator.com/item?id=30502908](https://news.ycombinator.com/item?id=30502908) - Feb 2022 (1 comment)

*Magic: The Gathering is Turing Complete* - [https://news.ycombinator.com/item?id=19847939](https://news.ycombinator.com/item?id=19847939) - May 2019 (192 comments)

*Magic: The Gathering Is Turing Complete* - [https://news.ycombinator.com/item?id=19744072](https://news.ycombinator.com/item?id=19744072) - April 2019 (1 comment)

*Magic: The Gathering Is Turing Complete (2012)* - [https://news.ycombinator.com/item?id=15712377](https://news.ycombinator.com/item?id=15712377) - Nov 2017 (115 comments)

*Magic: The Gathering Is Turing Complete (v5)* - [https://news.ycombinator.com/item?id=10317224](https://news.ycombinator.com/item?id=10317224) - Oct 2015 (31 comments)

*Magic: the Gathering is Turing Complete* - [https://news.ycombinator.com/item?id=4511384](https://news.ycombinator.com/item?id=4511384) - Sept 2012 (1 comment)

*Magic: the Gathering is Turing Complete* - [https://news.ycombinator.com/item?id=4506865](https://news.ycombinator.com/item?id=4506865) - Sept 2012 (1 comment)

Here's a demo: [https://www.youtube.com/watch?v=pdmODVYPDLA](https://www.youtube.com/watch?v=pdmODVYPDLA)

Didn't realize this before watching, but it's interesting that there's an incredibly complicated board state but the game state / actions are deterministic, e.g. the players don't have any choices about what to do once the machine is set up.

One my decks is actually built around getting the game into infinite combos which cannot end but that also don't kill anyone so the game ends in a tie. Same sort of thing. Always fun to pull off.

Most IRL play groups I’ve played with would count that as a loss for you (or, most likely, just not invite you back). And in competitive/regulated play you would timeout and lose. Not sure who these weirdos are that are stipulating to a draw against a deck that is unable to win.

Edit: I was wrong! I’ve only been playing competitively on Arena for years now. Per Rule 725.4 infinite loops are draws.

\> or, most likely, just not invite you back

Paper magic works with infinite combos on the basis that if you can prove them, you can "fast forward".

And someone with a draw infinite combo would definitely be welcome in some player circles (see the Jhonny definition).

Yeah you demonstrate the loop once and show that other whatever the output of the loop is (damage, mana, creature tokens, mill for your opponents) your board is in the same initial state so it can be repeated an arbitrary number of times (or it's forced to repeat in which case you're in a forced infinite loop).

The tournament rules, section 4.4, differ from the comprehensive rules and take precedence in tournaments:

[https://media.wizards.com/2023/wpn/marketing\_materials/wpn/M...](https://media.wizards.com/2023/wpn/marketing_materials/wpn/MTG_MTR_2023Dec4.pdf)

Does that mean the judge must solve the halting Problem?

The judge can solve the halting problem by ruling the outcome.

Seems somewhat analogous to draw by repetition in Chess

A friend of mine had a black/white deck that could loop you to death, it was no fun to play against, soon as he got the cards out he needed, he could (IIRC) banish a creature to the graveyard, then bring it back, repeatedly, and one of those actions inflicted damage on the opponent.

It was clever, but also beardy AF, to use a phrase from my days of WH40K

I used the same type of deck in Yu-Gi-Oh a few years ago. Something with fusion summoning elemental heroes which banished everything before bringing everything back. Wasn't all that good, but was somewhat fun to watch opponents realize the loop :)

Blue control deck?

Frustrating to play against if the loop is working, but often weak on killing power if it takes a lot of sacrificing to pay the upkeep.

I remember one game decades ago where I was slowly ground down by the tapping of a solitary Tim.

No it's a group hug Commander/EDH deck. We all win together!

But yes it does kinda frustrate people. That's the downside of liking Magic because of it being fun to break as a system and not because you want to smash giant monsters into each other.

I used to play with a guy who had a deck named “Judge Problems.”

I don’t know what all was in it (I never played against it) but remember being regaled with tales of what happens when Opalescence and Humility come into play together as part of an effect that puts a bunch of permanents into play all with the same time stamp.

Its the infamous "judge destroyer": [https://www.mtgthesource.com/forums/showthread.php?29732-Leg...](https://www.mtgthesource.com/forums/showthread.php?29732-Legacy-Judge-Destroyer-1-0)

Unbounded automatic combos can be any colors. One of the simplest is:

Aether Flash - Enchantment

Whenever a creature enters the battlefield, Aether Flash deals 2 damage to it.

Polyraptor - Creature - 5/5

Whenever Polyraptor is dealt damage, create a token that's a copy of Polyraptor.

I honestly thought calling Prodigal Sorcerers “Tim” was just a thing from a guy at my local comic store growing up.

Thanks for that memory.

Tim: There!

King Arthur: What, behind the rabbit?

Tim: It is the rabbit!

King Arthur: You silly sod!

Any infinite combo is of this form?

Like, if you have an infinite mana combo, you can just keep running the steps of it to block gameplay instead of playing your fireball

There's a difference between the player having to decide to keep looping by performing actions vs. the loop continuing on its own via triggers. The rules make a distinction and specifically don't allow someone to do the former indefinitely.

I was thinking about how Magic the Gathering has so many infinite combos. In a deck with a wide variety of cards, you're likely to be able to accidentally construct an infinite combo.

For those who don't play, the most iconic infinite combo involves two cards, the first says "Whenever you gain life, an opponent loses that much life.", the second card says "Whenever an opponent loses life, you gain that much life."

These cards, when combined, do nothing... until you gain a life or an opponent takes damage. Then their effects combined means a chain reaction that repeats until your opponents are dead and you have gained as much life as they had.

There's a variety of infinite combos in MTG. Some of them involve a creature that says "Tap to add mana to your mana pool" combined with another card that says "Pay mana to untap a creature", allowing you to tap and untap an infinite number of times.

Some infinite combos involve returning a card to your hand, and recasting it which gives you the resources you need to return it to your hand and recast again. Some infinite combos involve looping a card from your discard pile repeatedly.

There are no one-card infinite combos (that would likely not make it past the testers), but there are plenty of two-card infinite combos, and an combinatorically increasing number of three and four card infinite combos.

I think there is some similarity computationally speaking between turing completeness, and the ability to construct an infinite combo in a game like MTG. An infinite allows you (the player) to continue taking the same action over and over again, accumulating some game resource in the process. This bears resemblance to the infinite tape Turing envisioned, a way to hold data. Player actions are much analogous to the instruction set. Infinites that are optional for the player (not all infinites in MTG are optional once the pieces are on the board) can also stand in for conditional statements - a key requirement of turing completeness.

I'd be interested in seeing the bare minimum number of cards required to generate turing completeness. If anybody else knows more about this domain, I would love to hear their opinions.

For what it’s worth, it’s currently vintage cube season on Magic Online and you can draft a deck with multiple infinite combos without much effort. Sadly you have to do all the clicking so you might run out of time. Paper magic is much kinder because once you’ve demonstrated a loop you can basically assign infinite damage, gain infinite life, create infinite creatures etc without having to play it out.

What's funny is that after demonstrating the loop you still have to give a concrete number of times that you repeat it. You can't deal infinite damage, but you sure can do a googolplex damage.

Why go for such a small number? Raise Graham's Number to the power of itself -- at the very least.

[https://research.phys.cmu.edu/biophysics/2021/01/09/nobody-c...](https://research.phys.cmu.edu/biophysics/2021/01/09/nobody-comprehends-grahams-number)

I'm sure it can be entertaining to try and deal the largest possible finite amount of damage, which requires finding uncommonly large numbers.

Raising Google's Number to itself doesn't make it appreciably bigger. Instead you can pick TREE(3), or SCG(13), or Loader's Number which is about the largest famous one we know how to compute.

Beyond that there's the likes of Busy Beaver numbers and beyond that is Rayo's number.

Btw, Graham's Number is less than BB(49 bits) \[1\].

\[1\] [https://codegolf.stackexchange.com/questions/6430/shortest-t...](https://codegolf.stackexchange.com/questions/6430/shortest-terminating-program-whose-output-size-exceeds-grahams-number/263884#263884)

Please correct me if I'm wrong, but I thought an infinite combo that doesn't require user at interaction results in a draw. So your first example would be this, but the tapping one isn't. It's been years since i played however.

Infinite combos that require the player to opt into repeating an action will not end in a draw, because the player is expected to decide on the number of times the combo will repeat for.

Infinite combos that not optional but win you the game instantly, like the sanguine bond + exquisite blood combo I mentioned earlier, means you just win.

Infinite combos that are not optional but do not win you the game result in a draw.

\> Infinite combos that are not optional but do not win you the game result in a draw.

There's the 'donate an unsacrificable islandhome creature to an opponent who has no islands'.

Infinite amount of state based checks happen that are unavoidable, and cause the game to draw.

It depends. If the game state changes, say like a change in one player’s life total, then the loop won’t usually end in a draw. In the example above the opponent will eventually die.

In paper once a player has demonstrated a loop they must choose a number of times to repeat the loop and then the game is fast forwarded to the chosen end state. For example, a player might execute a loop that could gain them infinite life, but really they must choose a point to stop. Usually that player will choose 1,000,000,000,000 or another “essentially infinite” value and the game moves on.

There are infinite loops that can draw the game, but in a tournament game if one player can take an action that would end the loop, say by destroying one of the loop pieces, that player must take that action. Only if no player can end the loop does the game end in draw.

Is that true? If your opponent creates a loop but you have a spell that can end it, I don’t believe you’re compelled to cast it if you decide the draw is more favourable. Definitely don’t like that rule if it exists.

The Magic tournament rules cover this:

“Some loops are sustained by choices rather than actions. In these cases, the rules above may be applied, with the player making a different choice rather than ceasing to take an action. The game moves to the point where the player makes that choice. If the choice involves hidden information, a judge may be needed to determine whether any choice is available that will not continue the loop.”

Basically if a player has open information, like an activated ability, that could end the loop that player is not allowed to not use it to keep the loop going indefinitely. If that player instead has hidden information, i.e. a card in hand, that could end the loop any player can call a judge to confirm that and force the player to end the loop.

Note this doesn’t extend past cards in hand, though. If a player has some way to search their deck for a card that could end the loop, they are not forced to search and then play that card. At that level it moves from a player intentionally delaying the game with the resources at hand or in play to a judge dictating a player’s actions.

One nitpick, even if you have have an action that stops the loop, you are not necessarily required to take it, and can choose the draw instead.

719.5. No player can be forced to perform an action that would end a loop other than actions called for by objects involved in the loop.

The rule about choices in mandatory action comes into play for cases like an Oblivion Ring loop. If there is another valid target for the oblivion ring, you must choose it instead of forcing a draw. You can't say that you always choose the opponents Oblivion ring.

There was recently a ruling/clarification on exactly this kind of loop that has become relevant in competitive Pioneer format: [https://mtgrocks.com/mtg-ruling-may-cause-problematic-turn-t...](https://mtgrocks.com/mtg-ruling-may-cause-problematic-turn-three-combo-to-self-destruct/)

Yeah, I’ve come round to this because the base case is a zero mana activated ability that you never stop activating. That ought to give you a loss on time the moment your opponent calls a judge and you refuse to stop. More complex loops where you nevertheless choose to continue and can’t make a convincing case you’re progressing should be the same. The recent ruling just short circuits that discussion for one specific combo so I suppose I’m fine. For online play we have clocks and they should fix MTGO’s stack depth.

I’m still not convinced uncontrollable loops should be forcibly avoided. That seems unnatural and I’m not really against draws per se. I guess I’d just feel very angry in chess if I were forced to play a worse move and lose when a repetition leading to a draw was available.

In the recent ruling case (Amalia/Wildgrowth Walker), neither decision (mill cards or leave the same card on top of the deck) ends the loop, though. The loop will still continue with an empty library. The idea is that making the mill choice might eventually tempt the opponent to end the loop, if they have the ability - so that does seem a lot different to the *"you must choose to stop looping if you can"* case.

Don’t like that at all! I suppose some chess tournaments have “no draws before move X” but it’s a feeble rule that players easily overcome with repetitions etc. Forcing someone to take an action that they might deem worse for their chances seems wrong.

It’s mostly for time purposes. Nobody wants to wait another hour for the round to end because someone is trying to draw their game, which means those players potentially have to play yet another game. In reality it doesn’t happen that often.

In your games with your friends feel free to ignore this rule. If you made a deck that managed to pull it off I would think it was cool.

The rule is not precisely that, it is only if the mandatory actions of the loop offer you the choice, that you must eventually choose one that stops the loop. Classic infinite loop example is 3 oblivion rings exiling each other. If there is another valid target, you must choose it.

There’s never a time where players can’t interact - just passing priority or putting triggered abilities on the stack are actions. And each time this could happen the game checks state based actions to see if someone has won. That said, if both players have no options or just pass (which is more pronounced online) then you can end up in inescapable loops that cause draws. A classic example is this Luis Scott-Vargas game:

[https://youtu.be/AGXG5rNe\_tI?feature=shared](https://youtu.be/AGXG5rNe_tI?feature=shared)

You just described my Vampire deck, using Vito and Exquisite Blood.

Personal favorite is Squirrel's Nest + Earthcraft. Coat of Arms is optional fun lol

[https://commanderspellbook.com/](https://commanderspellbook.com/)

\> There are no one-card infinite combos

There are. Basalt Monolith has "tap: add 3 mana" and "3 mana: untap". It doesn't actually have any net effect without some other card triggering on that or modifying it somehow, but it actually can do an infinite number of actions by itself as one card.

Obligatory "we finally broke basalt monolith"

This card is a staple for so many infinite combos because, just like you're saying, it adds mana (the biggest restriction to pulling off broken effects), and it can be an engine for any number of complimentary effects that trigger off tapping or untapping or reducing costs or copying triggers.

My favourite combo ever was infinite 5/5 dinosaurs

This paper gets cited in almost every Magic thread, of which there have been 2 recently that may interest this audience:

[https://news.ycombinator.com/item?id=38525978](https://news.ycombinator.com/item?id=38525978) *(I hacked Magic the Gathering: Arena for a 100% win rate)*

[https://news.ycombinator.com/item?id=38533105](https://news.ycombinator.com/item?id=38533105) *(Fine-tuning Mistral 7B on Magic the Gathering Draft)*

and to "one stop shopping" this linkathon, today someone helpfully posted a link to Forge, the open source (GPLv3) M:TG engine: [https://news.ycombinator.com/item?id=38651346](https://news.ycombinator.com/item?id=38651346)

A great read on that topic is Gwern's Surprisingly Turing Complete: [https://gwern.net/turing-complete](https://gwern.net/turing-complete)

[https://www.mtggoldfish.com/deck/3933484#paper](https://www.mtggoldfish.com/deck/3933484#paper)

This deck has gotten cheaper in the last couple years, looks like it's currently $2400 to build.

Is it competitive?

The odds of getting the combo off are extremely low so probably not.

Definitely not. I don’t think you could actually pull this off against another pile of cards trying to play traditionally unless your opponents are in on the joke.

It's a winning hand, but your odds of drawing it are inverse galactic.

it's a winning hand if the other player doesn't counter any of your cards before you exile their hand and library ;-)

\> In \[4\], the author presents a Magic: The Gathering end- game that embeds a universal Turing machine. However, this work has a major issue: it’s not quite deterministic. At several points in the simulation, players have the ability to stop the computation at any time by opting to decline to use effects that say “may.” For example, Kazuul Warlord reads “Whenever Kazuul Warlord or another Ally enters the battlefield under your control, you may put a +1/ + 1 counter on each Ally you control.” Declining to use this ability will interfere with the Turing machine, either causing it to stop or causing it to perform a different calculation from the one intended. The construction as given in Churchill \[4\] works under the assumption that all players that are given the option to do something actually do it, but as the author notes it fails without this assumption. Attempts to correct this issue are discussed in Churchill et al. \[5\]. In this work, we solve this problem by reformulating the construction to exclusively use cards with mandatory effects

So they only use cards with mandatory effects/triggers. Doesn't this mean that this whole work if flawed because it only applies to a subset of MTG cards?

Waiting for someone to port Pokemon to MTG and then run that one exploit that allows you to execute arbitrary code using in-game commands in order to play Pong.

The exploit in question: [https://tasvideos.org/3358M](https://tasvideos.org/3358M)

Pokémon Yellow Total Control Hack: [https://youtu.be/p5T81yHkHtI](https://youtu.be/p5T81yHkHtI)

I've wondered can a game similar to MTG be created but cards for real-life, as a means to educate those who gravitate to devour such information and strategy as a force to first spot and raise the alarms loudly enough to counter any tyrannical-authoritarian tactics?

Perhaps a hybrid card-board game with a foundation similar to Monopoly?

It's something I want to eventually help create, though I'm not in a position to put a proper effort needed to get the base created.

There are a few computer games that simulate this evolution though, no? But perhaps without the level of detail such as slippery-slope policies that an opponent \[whether directed by a player or machine\] may try to pass their parliament, other tactics like playing a card that successfully "manufactures consent" with the population to move various dials/levers ahead to implement turnkey authoritarianism - to then at a certain point in future once enough foundation is laid, the population not paying adequate attention to the empire - entertained excessively, the player-machine can then execute on locking down their society?

I'm not sure if this is what you have in mind, but have you seen Nomic, or perhaps even the boardgame Secret Hitler is relevant?

I had heard of Secret Hitler before - Nomic looks great too! Thanks for sharing.

And yes, exactly what I am thinking but an interconnected system of game play-learning that is able to incorporate all these various angles of attack/approach.

I think the foundation of our education needs to focus on fortifying against the peace-freedom vs. tyranny - and some systems where all of these similar parts connect in a highly complex game, that perhaps kids can only play certain parts of it - and then where there will become "power users" who are capable of encapsulating and navigating all of it.

There's some irony (?) in that this system could also arguably be a playbook to teach tyrant-wannabes what to do, a strategy playbook to some degree, however shining light on everything, witnessing it, and not giving into fear is the answer.

MoTG also had the concept of a stack, e.g. if I have two mishra's factory ([https://gatherer.wizards.com/pages/card/details.aspx?name=Mi...](https://gatherer.wizards.com/pages/card/details.aspx?name=Mishra%27s%20Factory)), activate one to make it a 2/2, you decide to lightning bolt it (deals 3 damage), I then tap the other to make to a 3/3, then tap this one to make it a 4/4.

And phases and timing: if you don't do anything, and I tap to attack, and THEN you lightning bolt it... then I can no longer tap it to... yeah, you get the idea.

Fun times.

Always found it funny how "the stack", which exists in MTG to make it more deterministic and comprehensible, also makes this kind of arbitrary complexity easier to reason about and pull off.

Also, how consistent the rules are overall. e.g. the players themselves count as planeswalkers.

it's Turing complete as long as the opposing player doesn't cast Force of Will ;-)

I don't play MTG, but wouldn't that be like saying that your CPU is Turing-complete as long as you don't install a system that lets you manually modify data while it is in the CPU? In other words, it's Turing-complete but has a manual override to allow you to change/corrupt your data.

It was mostly a tongue-in-cheek comment since Force of Will is notorious from preventing opponents from playing their perfectly structured plan even when they think they've accounted for the target player *not* being able to cast an instant counter (which traditionally was the case when the target player has no mana available, since most counters cost mana)...

so I guess I'm just saying it's not guaranteed to work every time you draw the perfect Turing-complete hand setup because you may be foiled by a counter you don't expect

MTG has "stacks", and cards are cast-and-resolved in two separate steps. Which means cards are all cast in sequenced but then resolved in the reverse order they were cast. For example, if Player 1 casts spells A, B, C, and then Player 2 casts instant card D which reads ("cancel the prior spell") in response to Player 1's casting of C, then D resolves by popping C off the stack without C impacting the game at all, then B and A resolve

The way this is set up, if the player in question starts their turn with the needed cards, and isn’t under any enchantments, then after the turn finishes neither player can make any decisions at all. So if the player in question has the first turn, they can choose to start the machine, which will proceed to either finish in a victory, or not finish causing a draw.

one can cast Force of Will (or any \`instant\` card, really, but that's just a well-known counter that doesn't (necessarily) cost any mana) during the other player's turn. so even if the other player drew the perfect hand, one could counter any of his spells to break the combo early and prevent it from working

there's a chance they could recover from it, but one could always cast another Force of Will...

Hilarious that the first turn is to get infinite mana and then draw the entire library into your hand

so if I kept redrawing neighbors I'd find that one woman, that could engineer my dream child (after we made sweet sweet love)?

that is, if another neighbor didn't chose to redraw that exact time when I didn't ...

wait. are you saying I need to get rid of that one neighbor? wait.
