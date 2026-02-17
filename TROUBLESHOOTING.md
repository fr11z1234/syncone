# SyncONE – Troubleshooting

## Non-hosts get stuck on "Syncing" when launching after a pull

When everyone uses the **exact same** synced save and the game (Schedule I) is launched, the **host** often loads fine, but **people who join** can get stuck on screens like "Syncing StorageEntity", "Syncing NPC Behavior", or "Syncing NPC Inventories". This is a known Schedule I multiplayer issue when the save contains session state from the last time it was played.

### Why this happens

The save folder structure looks like this:

- **Save path** (what you point SyncONE at):  
  `...\AppData\LocalLow\TVGS\Schedule I\Saves\<YourSteamID>\`
- Inside that you have **SaveGame_1**, **SaveGame_2**, etc.
- Inside each **SaveGame_*** you typically have:
  - **Money.json**, **Time.json**, **Game.json**, **Metadata.json**, etc.
  - **Variables/** – game state flags (e.g. `HasExitedRV.json`)
  - **Players/** – character/inventory data

When the same save is shared:

1. **Session state** – The save was last used in a multiplayer session. That state (who was host, session IDs, etc.) can be stored in **Variables/** or in JSON like **Game.json** / **Metadata.json**. When a **non-host** loads that save and joins, the game may still think it is "resuming" a session and wait for the **original** host, so joiners get stuck on "Syncing".
2. **HasExitedRV** – Schedule I uses a variable `HasExitedRV` in **Variables/** to handle rejoin/session logic. If it’s missing or wrong after a pull, the game can get stuck in a bad sync state.

So the issue is **not** SyncONE copying files wrongly; it’s that the **save content** carries over session/host state that confuses the game when someone other than the original host runs it.

### What SyncONE does to help

After every **Save** pull, SyncONE now writes **HasExitedRV.json** into each **SaveGame_*/Variables/** folder in the pulled save. That matches the usual community fix for the "Multiplayer Rejoin Bug" and often helps joiners not get stuck on "Syncing" the first time they launch after a pull.

### If joiners are still stuck

1. **Host creates the lobby first**  
   The person who will host should start the game, load the synced save, and **create** the multiplayer lobby. Others then **join** that lobby. Avoid the joiner loading the save and "hosting" first if they weren’t the original host.

2. **Manual HasExitedRV fix**  
   If SyncONE’s automatic fix isn’t enough, the joiner can do it manually:
   - Go to:  
     `%LocalAppData%Low\TVGS\Schedule I\Saves\<SteamID>\SaveGame_<N>\Variables`
   - Create (or overwrite) **HasExitedRV.json** with:
   ```json
   {"DataType": "VariableData","DataVersion": 0,"GameVersion": "0.0.0","Name": "HasExitedRV","Value": "True"}
   ```
   - Use the correct SaveGame slot number (`<N>`) and your Steam ID folder.

3. **Game bugs**  
   "Stuck on Syncing" is a known Schedule I multiplayer bug (e.g. [Steam discussion](https://steamcommunity.com/app/3164500/discussions/1/795584078204168598/)). If it keeps happening, try game restarts, different host, or wait for a game update.

### Save structure reference

| Path (under your save folder) | Purpose |
|-------------------------------|--------|
| **SaveGame_1**, **SaveGame_2**, … | Save slots. SyncONE syncs the whole folder (all slots). |
| **SaveGame_*/Money.json** | Used by SyncONE to compare progress (LifetimeEarnings). |
| **SaveGame_*/Variables/** | Game state flags; can affect multiplayer (e.g. HasExitedRV). |
| **SaveGame_*/Players/** | Character/inventory data. |

The **parent** folder of the save (the one with your Steam ID) is **not** synced by SyncONE; each PC has its own. Only the **contents** (SaveGame_*, etc.) are shared, so everyone can have the same world even though their paths use different Steam IDs.
