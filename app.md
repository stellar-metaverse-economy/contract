# App Architecture — Game Client

The game client is a browser-based strategy game that connects to the Soroban contracts via the backend API and directly via Freighter wallet for transaction signing.

---

## Stack

| Layer | Tech |
|---|---|
| Game rendering | Phaser 3 / Babylon.js |
| Frontend framework | Next.js (App Router) |
| State management | Zustand |
| Wallet | Freighter (Stellar browser extension) |
| UI | Tailwind CSS |
| Real-time | WebSockets |
| API client | GraphQL (urql) + REST fetch |

---

## Directory Structure

```
apps/
├── game-client/
│   ├── public/
│   └── src/
│       ├── components/       # Shared UI components (modals, panels, buttons)
│       ├── scenes/           # Phaser game scenes (WorldMap, RegionView, Battle)
│       ├── ui/               # HUD overlays, sidebars, notifications
│       ├── map/              # Map rendering, region tiles, zoom/pan
│       ├── economy/          # Yield display, share calculator, revenue charts
│       ├── wallet/           # Freighter connect, sign tx, address display
│       ├── hooks/            # useWallet, useRegion, useMarketplace, useYield
│       ├── stores/           # Zustand stores (player, regions, market, ws)
│       ├── shaders/          # GLSL shaders for map effects
│       └── multiplayer/      # WebSocket client, event handlers
│
├── admin-dashboard/          # Region management, yield deposits, contract admin
└── analytics-dashboard/      # Economy charts, ownership distribution, trade volume
```

---

## Connection Flow

```
User opens app
      │
      ▼
Connect Freighter wallet
      │  wallet_address extracted
      ▼
POST /auth/connect  (sign challenge with Freighter)
      │  receive JWT
      ▼
Load world state via GraphQL
      │  regions, player holdings, listings, proposals
      ▼
Open WebSocket connection  wss://ws.game.io?token=<jwt>
      │  subscribe to live events
      ▼
Game loop starts (Phaser scene)
```

---

## Wallet Integration

All on-chain transactions are signed by the player via Freighter. The backend never holds player private keys.

```ts
// src/wallet/freighter.ts
import { getPublicKey, signTransaction, isConnected } from "@stellar/freighter-api";

export async function connectWallet() {
  if (!(await isConnected())) throw new Error("Freighter not installed");
  return getPublicKey();
}

export async function signTx(xdr: string, network: "TESTNET" | "PUBLIC") {
  return signTransaction(xdr, { network });
}
```

**Transaction flow for any contract call:**

```
Frontend builds XDR (via stellar-sdk)
      │
      ▼
Freighter prompts player to sign
      │
      ▼
Signed XDR submitted to Horizon
      │
      ▼
Backend indexing-service picks up the event
      │
      ▼
UI updates via WebSocket push
```

---

## Zustand Stores

```ts
// stores/player.ts
{ address, jwt, holdings, claimable }

// stores/regions.ts
{ regions: Region[], selected: Region | null }

// stores/market.ts
{ listings: Listing[], myListings: Listing[] }

// stores/ws.ts
{ connected, subscribe(event, handler), unsubscribe }
```

---

## Key Hooks

```ts
// Connect wallet and authenticate
useWallet() → { address, connect, disconnect, signTx }

// Load and watch a region
useRegion(regionId) → { region, shares, claimable, loading }

// Marketplace actions
useMarketplace() → { listings, listShares, buyShares, cancelListing }

// Yield actions
useYield(regionId) → { claimable, claim, history }

// Governance
useGovernance(regionId) → { proposals, createProposal, vote }
```

---

## Screens & Scenes

| Screen | Description |
|---|---|
| `WorldMap` | Phaser scene — zoomable map of all regions, color-coded by type |
| `RegionView` | Detail panel — stats, share ownership, yield history, governance |
| `Marketplace` | Browse listings, buy/sell shares, price history chart |
| `Portfolio` | Player's holdings, claimable yield, transaction history |
| `Governance` | Active proposals, vote, create proposal |
| `Treasury` | Region treasury balance (governor view) |

---

## API Integration

### GraphQL (read)

```ts
// src/economy/queries.ts
const REGION_QUERY = gql`
  query Region($id: Int!) {
    region(id: $id) {
      name totalShares productivityScore taxRate revenuePool
    }
  }
`;

const PLAYER_QUERY = gql`
  query Player($address: String!) {
    player(address: $address) {
      holdings { regionId shares claimable }
    }
  }
`;
```

### REST (actions that go through backend before hitting chain)

```ts
// List shares — backend validates, builds XDR, returns for signing
POST /marketplace/list
Body: { regionId, shares, pricePerShare }
Response: { xdr }  // player signs via Freighter, then submits

// Claim yield
POST /yield/claim
Body: { regionId }
Response: { xdr }

// Create proposal
POST /governance/propose
Body: { regionId, description, votingPeriodLedgers }
Response: { xdr }
```

### WebSocket (real-time)

```ts
// src/multiplayer/ws.ts
const ws = useWsStore();

ws.subscribe("region.yield_deposited", ({ regionId, amount }) => {
  updateRegionPool(regionId, amount);
  showToast(`Yield deposited: ${amount} stroops`);
});

ws.subscribe("marketplace.listing_sold", ({ listingId }) => {
  refreshListings();
});

ws.subscribe("governance.finalized", ({ proposalId, status }) => {
  updateProposal(proposalId, status);
});
```

---

## Environment Variables

```env
NEXT_PUBLIC_API_URL=https://api.game.io
NEXT_PUBLIC_WS_URL=wss://ws.game.io
NEXT_PUBLIC_STELLAR_NETWORK=TESTNET
NEXT_PUBLIC_HORIZON_URL=https://horizon-testnet.stellar.org

NEXT_PUBLIC_CONTRACT_REGION_FACTORY=C...
NEXT_PUBLIC_CONTRACT_FRACTIONAL_OWNERSHIP=C...
NEXT_PUBLIC_CONTRACT_YIELD_DISTRIBUTOR=C...
NEXT_PUBLIC_CONTRACT_MARKETPLACE=C...
NEXT_PUBLIC_CONTRACT_GOVERNANCE_DAO=C...
NEXT_PUBLIC_CONTRACT_TREASURY=C...
```

---

## Admin Dashboard

Separate Next.js app for game operators. Connects to the same backend with an admin JWT.

**Features:**
- Create regions (`region-factory.create_region`)
- Mint initial shares (`fractional-ownership.mint`)
- Deposit revenue into yield pools (`yield-distributor.deposit_revenue`)
- Set share snapshots for yield distribution
- Upgrade region productivity
- Treasury deposits and withdrawals

---

## Dev Setup

```bash
cd apps/game-client
npm install
npm run dev        # http://localhost:3000
```

Install Freighter browser extension and switch to **Stellar Testnet** before connecting.
