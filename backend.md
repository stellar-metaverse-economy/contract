# Backend Architecture

The backend sits between the game client and the Soroban smart contracts. It handles auth, real-time communication, economy simulation, contract indexing, and scheduled reward distribution.

---

## Stack

| Service | Language | Purpose |
|---|---|---|
| `api-gateway` | Node.js + Fastify | Auth, routing, rate limiting, GraphQL |
| `game-engine` | Rust / Go | World simulation, NPC behavior, battle calculations |
| `economy-engine` | Rust / Go | Region productivity, inflation, tax, reward epochs |
| `websocket-server` | Node.js | Real-time game events, price feeds, governance updates |
| `indexing-service` | Node.js | Soroban event indexer → PostgreSQL |
| `governance-service` | Node.js | Proposal sync, vote tallying, finalization triggers |
| `reward-engine` | Rust / Go | Epoch-based yield snapshots, batch distribution calls |
| `matchmaking` | Node.js | PvP pairing, war declarations |
| `ai-engine` | Python | NPC behavior, dynamic economy modeling |
| `cron-workers` | Node.js | Scheduled epochs, seasonal resets, productivity ticks |

---

## Directory Structure

```
backend/
├── api-gateway/          # Entry point — auth, GraphQL, REST, rate limiting
├── game-engine/          # World state, NPC logic, battle resolution
├── economy-engine/       # Productivity scores, tax collection, epoch rewards
├── ai-engine/            # AI-driven NPC economies and region events
├── matchmaking/          # PvP war pairing and alliance logic
├── websocket-server/     # Real-time push to game clients
├── reward-engine/        # Snapshot shares, call yield-distributor contract
├── governance-service/   # Sync proposals/votes from chain, trigger finalize()
├── indexing-service/     # Listen to Soroban events, write to DB
└── cron-workers/         # Scheduled jobs: epochs, resets, ticks
```

---

## Data Flow

```
Game Client
    │
    ▼
api-gateway  ──────────────────────────────────────────────────────┐
    │                                                               │
    ├── REST / GraphQL queries ──► indexing-service DB (read)       │
    │                                                               │
    ├── Mutations ──► game-engine / economy-engine                  │
    │                                                               │
    └── WebSocket upgrade ──► websocket-server                      │
                                                                    │
Soroban Contracts ◄─────────────────────────────────────────────────┘
    │   (region-factory, fractional-ownership, marketplace,
    │    yield-distributor, governance-dao, treasury)
    │
    ▼
indexing-service  (Horizon + custom event listener)
    │
    ▼
PostgreSQL + Redis
    │
    ├── indexing-service ──► serves read queries via api-gateway
    └── reward-engine    ──► reads snapshots, calls deposit_revenue / claim
```

---

## API Gateway

**Base URL:** `https://api.game.io`

### Auth

```
POST /auth/connect
Body: { wallet_address, signed_challenge }
Response: { jwt_token }
```

All subsequent requests require `Authorization: Bearer <jwt_token>`.

### REST Endpoints

```
GET  /regions                        # List all regions
GET  /regions/:id                    # Region detail + productivity
GET  /regions/:id/shares/:address    # Player share balance
GET  /marketplace/listings           # Active listings
GET  /governance/proposals           # Active proposals
GET  /players/:address               # Player profile + holdings
GET  /players/:address/claimable     # Pending yield per region
```

### GraphQL

```graphql
query {
  region(id: 1) {
    name
    totalShares
    productivityScore
    taxRate
    revenuePool
  }
  player(address: "G...") {
    holdings { regionId shares claimable }
    proposals { id description status }
  }
}
```

---

## WebSocket Events

Connect: `wss://ws.game.io?token=<jwt>`

| Event | Direction | Payload |
|---|---|---|
| `region.yield_deposited` | server → client | `{ regionId, amount }` |
| `marketplace.listing_created` | server → client | `{ listingId, regionId, price }` |
| `marketplace.listing_sold` | server → client | `{ listingId, buyer, amount }` |
| `governance.proposal_created` | server → client | `{ proposalId, regionId }` |
| `governance.finalized` | server → client | `{ proposalId, status }` |
| `game.region_updated` | server → client | `{ regionId, productivityScore }` |

---

## Indexing Service

Listens to Soroban contract events via Horizon and writes to PostgreSQL.

**Tracked events:**

| Contract | Event | Action |
|---|---|---|
| `region-factory` | `REG_NEW` | Insert region row |
| `fractional-ownership` | `TRANSFER` | Update ownership table |
| `yield-distributor` | `YIELD` | Update revenue_pool, notify via WS |
| `marketplace` | `LISTED`, `SOLD` | Insert/update listing rows |
| `governance-dao` | `PROPOSED`, `VOTED` | Insert proposal/vote rows |
| `treasury` | `DEPOSIT`, `WITHDRAW` | Update treasury balance |

---

## Database Schema

```sql
-- Players
CREATE TABLE players (
  id             SERIAL PRIMARY KEY,
  wallet_address VARCHAR(64) UNIQUE NOT NULL,
  username       VARCHAR(64),
  reputation     INT DEFAULT 0,
  created_at     TIMESTAMPTZ DEFAULT NOW()
);

-- Regions (mirrored from chain)
CREATE TABLE regions (
  id                 BIGINT PRIMARY KEY,
  name               VARCHAR(128),
  region_type        INT,
  total_shares       NUMERIC,
  productivity_score INT,
  tax_rate           INT,
  governor           VARCHAR(64),
  revenue_pool       NUMERIC DEFAULT 0,
  updated_at         TIMESTAMPTZ DEFAULT NOW()
);

-- Ownership snapshots
CREATE TABLE ownerships (
  player_id  INT REFERENCES players(id),
  region_id  BIGINT REFERENCES regions(id),
  shares     NUMERIC NOT NULL,
  PRIMARY KEY (player_id, region_id)
);

-- Marketplace listings
CREATE TABLE listings (
  id               BIGINT PRIMARY KEY,
  seller           VARCHAR(64),
  region_id        BIGINT,
  shares           NUMERIC,
  price_per_share  NUMERIC,
  active           BOOLEAN DEFAULT TRUE,
  created_at       TIMESTAMPTZ DEFAULT NOW()
);

-- Yield payouts
CREATE TABLE payouts (
  id         SERIAL PRIMARY KEY,
  player_id  INT REFERENCES players(id),
  region_id  BIGINT,
  amount     NUMERIC,
  claimed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Governance
CREATE TABLE proposals (
  id          BIGINT PRIMARY KEY,
  region_id   BIGINT,
  proposer    VARCHAR(64),
  description TEXT,
  votes_for   NUMERIC DEFAULT 0,
  votes_against NUMERIC DEFAULT 0,
  status      VARCHAR(16) DEFAULT 'active',
  end_ledger  BIGINT,
  created_at  TIMESTAMPTZ DEFAULT NOW()
);
```

---

## Reward Engine (Epoch Flow)

Runs on a cron schedule (e.g. every hour):

```
1. Snapshot ownership from fractional-ownership contract
2. Call economy-engine to calculate region revenue for the epoch
3. Call yield-distributor.deposit_revenue(region_id, amount) for each region
4. Update indexing DB with new revenue_pool values
5. Emit websocket event: region.yield_deposited
```

Players then call `claim()` themselves via the frontend, or the reward-engine can batch-trigger claims.

---

## Environment Variables

```env
# Stellar
STELLAR_NETWORK=testnet                        # testnet | mainnet
HORIZON_URL=https://horizon-testnet.stellar.org
CONTRACT_REGION_FACTORY=C...
CONTRACT_FRACTIONAL_OWNERSHIP=C...
CONTRACT_YIELD_DISTRIBUTOR=C...
CONTRACT_MARKETPLACE=C...
CONTRACT_GOVERNANCE_DAO=C...
CONTRACT_TREASURY=C...
ADMIN_SECRET_KEY=S...

# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/game

# Redis
REDIS_URL=redis://localhost:6379

# Auth
JWT_SECRET=...
JWT_EXPIRY=7d

# Server
API_PORT=3000
WS_PORT=3001
```

---

## Security

- All contract-mutating calls require a valid JWT tied to the player's wallet address
- Admin-only contract functions (`mint`, `deposit_revenue`, `set_shares`) are called only by backend service accounts, never exposed to the client
- Rate limiting on all endpoints via api-gateway (100 req/min per IP)
- Snapshot-based reward accounting prevents flash-loan abuse
- Ownership caps enforced at the contract level
