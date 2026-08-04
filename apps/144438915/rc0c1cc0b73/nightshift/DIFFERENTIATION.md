# What makes this different (the Cecilia answer)

Cecilia's standing objection is the right one: "we can already do this on the Aomi web app." For most demos she is correct, and this document is the honest account of where Nightshift is genuinely different and where it is not.

## Where the chat already wins

chat.aomi.dev, today, already:

- runs an agent on Solana,
- simulates a transaction before signing it,
- refuses a lookalike token.

So a demo whose whole pitch is "watch it simulate before signing and refuse a fake SOL" adds nothing. That is table stakes we already ship. If Nightshift were only that, Cecilia is right to wave it off.

## The one thing the chat structurally cannot do

The chat is **synchronous**. You are present. You approve each transaction in the moment. It is guarding the spend, one message at a time, with a human in the loop.

The chat has no answer for **"do this every night while I am asleep and not watching."** That needs a different primitive:

> a bound you read and grant **once**, that then lets an agent act **unattended** on a schedule, and that you could **audit before granting** because its powers were fixed at build time.

That is the other half of Aomi's own wedge. Compile-time permissions are exactly what make an unattended delegation safe enough to hand out. The chat guards the spend you approve. Nightshift makes the guard itself the product.

**One line for the room:** they guard the spend, we guard the build.

## What Nightshift actually contributes

1. **The permission envelope as a readable spec, up front.** The entire left column is the box: the mints it may touch (allowlisted by mint, not ticker), the max per action, the max per day, the one venue, the max slippage, and the namespace permissions it was built with. The chat never renders the box. This is the real, safe, shipped differentiator. You read it before you grant it.

2. **Enforcement you can watch, not take on faith.** `evaluate()` is a deterministic function, not a model call. Set a rule that exceeds the box and it is refused before it is ever quoted. The seeded ledger shows the boundary turning away an oversized buy and a lookalike mint.

3. **The unattended path, described honestly.** In production a delegated grant signs server-side, inside this same envelope. Nightshift does not auto-run that signer in the demo, on purpose, because unattended real-money signing is precisely the thing users are afraid of, and the point of the app is that you would only enable it after reading the box.

## The unattended signer is real, and here is the proof

The mechanism is not hand-waving. During the build, a real grant key produced a real, cryptographically valid signature over a real Solana transaction, with no human in the loop:

- grant key: `5NMZZsw73RV5EgsovLDo6xUa5MXeexGPZMxqfNXQpiyo`
- signature: `2o73xoAeuRjKR6CSt6axE54BuPs3FgYeYQDisf4FWtWE1XB884JAHryS6Y268sYEL9Kej4E9fZaLATSnZFPMYea1`
- cluster: devnet

The signer works. What the shipped app deliberately does not do is loop it automatically, because an agent that signs on a timer with no human present is the exact risk the whole product is built to make legible first. That restraint is the message, not a gap.

## How to run the demo for Cecilia

1. Open with the box. Read the left column out loud. "This is everything it can ever do. Fixed in code."
2. Type a rule. Show the live Jupiter quote for the next action.
3. Type "put $500 into SOL." Watch it refused in real time, with the exact clause it broke.
4. Land on the line: the chat guards the spend you approve; this is the bound you grant once and can read first. That is the half the chat does not expose.

Do not pitch simulate-before-sign as the novelty. Pitch the readable, grantable box.
