# Helixa <> Aomi Demo Note

Helixa gives Aomi agents a trust lookup layer.

Instead of treating agent identity as a pile of separate NFTs, Helixa exposes a richer AgentDNA profile:

- core identity
- Aura/personality
- traits and credentials
- owner and agent wallets
- services and communication endpoints
- Cred score and tier

Aomi App V0 is read-only and safe to load broadly. It lets any Aomi agent answer:

- Who is this agent?
- What wallet owns it?
- What Cred tier does it have?
- What services does it expose?
- Is this agent a good routing candidate?

Future write flows should be designed separately after V0 is reviewed.
