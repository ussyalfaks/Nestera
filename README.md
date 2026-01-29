# Nestera  
**Decentralized Savings & Investment Platform on Stellar**

Nestera is a decentralized savings and investment protocol built on **Stellar using Soroban smart contracts**. It enables individuals and communities to save transparently using stablecoins, with flexible, locked, goal-based, and group savings mechanisms enforced fully on-chain.

The project solves the problem of opaque, centralized savings platforms in emerging markets by providing a non-custodial, transparent alternative where users maintain full control of their funds. Nestera is designed for developers, contributors, and financial communities interested in building open, composable savings infrastructure using low-fee, fast-finality blockchain primitives.

---

## 🚀 Core Features

- Non-custodial savings via Soroban smart contracts
- Flexible and locked savings with deterministic interest logic
- Goal-based savings with automated milestones
- Group savings pools with shared rules and governance
- Native USDC-based savings on Stellar testnet
- Web interface for seamless contract interaction

---

## 🏗 Architecture Overview

- **Frontend (`apps/web`)**  
  Next.js application for interacting with Nestera smart contracts. Provides user interface for creating savings accounts, depositing funds, and tracking progress.

- **Backend (`apps/api`)**  
  Node.js API for off-chain services such as indexing contract events, sending notifications, managing user metadata, and aggregating analytics.

- **Smart Contracts (`contracts/`)**  
  Soroban smart contracts written in Rust that manage all savings logic, fund custody, interest calculations, and withdrawal rules.

---

## 📁 Repository Structure
```text
/
├── apps/
│   ├── web/              # Next.js frontend
│   └── api/              # Node.js backend API
├── contracts/            # Soroban smart contracts (Rust)
├── packages/             # Shared utilities and types
├── scripts/              # Deployment and automation scripts
├── tests/                # Integration and E2E tests
└── README.md
```

---

## 🛠 Setup Instructions

### Prerequisites

Before you begin, ensure you have the following installed:

- **Node.js** (v18 or higher) - [Download](https://nodejs.org/)
- **npm** or **yarn** - Comes with Node.js
- **Rust** (stable toolchain) - [Install](https://rustup.rs/)
- **Soroban CLI** - Instructions below
- **Stellar testnet account** - We'll create this in setup

### Installation Overview

1. Clone the repository
2. Set up smart contracts
3. Set up backend API
4. Set up frontend
5. Run tests

---

## 📦 1. Clone the Repository
```bash
git clone https://github.com/your-org/nestera.git
cd nestera
```

---

## 🔗 2. Smart Contracts Setup (Soroban)

### Install Soroban CLI
```bash
cargo install --locked stellar-cli --features opt
```

Or use the install script:
```bash
curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh | sh
```

Verify installation:
```bash
stellar --version
```

### Configure Stellar Testnet
```bash
stellar network add --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

### Generate Identity & Fund Account
```bash
stellar keys generate --global alice --network testnet
```

Get your address:
```bash
stellar keys address alice
```

Fund your account using Friendbot:
```bash
curl "https://friendbot.stellar.org?addr=$(stellar keys address alice)"
```

Verify balance:
```bash
stellar account balance --id alice --network testnet
```

### Build Contracts
```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
```

### Deploy Contracts
```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/nestera_contract.wasm \
  --source alice \
  --network testnet
```

Save the contract ID output - you'll need it for frontend and backend setup.

### Initialize Contract (if required)
```bash
stellar contract invoke \
  --id YOUR_CONTRACT_ID \
  --source alice \
  --network testnet \
  -- initialize \
  --admin $(stellar keys address alice)
```

---

## 🖥 3. Backend Setup (Node.js API)
```bash
cd apps/api
npm install
```

### Create Environment File

Create `.env` in `apps/api/`:
```env
PORT=3001
NODE_ENV=development

# Stellar Network
STELLAR_NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
HORIZON_URL=https://horizon-testnet.stellar.org

# Contract
CONTRACT_ID=YOUR_DEPLOYED_CONTRACT_ID

# Database (if using)
DATABASE_URL=postgresql://user:password@localhost:5432/nestera

# Optional
REDIS_URL=redis://localhost:6379
```

### Run Database Migrations (if applicable)
```bash
npm run migrate
```

### Start Backend Server
```bash
npm run dev
```

Backend should now be running at `http://localhost:3001`

### Verify Backend
```bash
curl http://localhost:3001/health
```

---

## 🌐 4. Frontend Setup (Next.js)
```bash
cd apps/web
npm install
```

### Create Environment File

Create `.env.local` in `apps/web/`:
```env
# Stellar Network
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
NEXT_PUBLIC_HORIZON_URL=https://horizon-testnet.stellar.org

# Contract
NEXT_PUBLIC_CONTRACT_ID=YOUR_DEPLOYED_CONTRACT_ID

# Backend API
NEXT_PUBLIC_API_URL=http://localhost:3001

# Wallet Connect (optional)
NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID=your_project_id
```

### Run Development Server
```bash
npm run dev
```

Frontend should now be running at `http://localhost:3000`

### Build for Production
```bash
npm run build
npm start
```

---

## 🧪 5. Running Tests

### Contract Tests
```bash
cd contracts
cargo test
```

### Backend Tests
```bash
cd apps/api
npm test
```

Run with coverage:
```bash
npm run test:coverage
```

### Frontend Tests
```bash
cd apps/web
npm test
```

Run E2E tests (requires running backend and deployed contracts):
```bash
npm run test:e2e
```

### Integration Tests

From project root:
```bash
npm run test:integration
```

---

## 🌍 Network Configuration

### Testnet

- **Network Passphrase:** `Test SDF Network ; September 2015`
- **RPC URL:** `https://soroban-testnet.stellar.org:443`
- **Horizon URL:** `https://horizon-testnet.stellar.org`
- **Friendbot:** `https://friendbot.stellar.org`

### Contract Addresses (Testnet)

- **Main Savings Contract:** `CXXXXXX...` (Update after deployment)
- **USDC Token:** `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA`

---

## 🐛 Troubleshooting

### Contract Deployment Fails

**Error:** `insufficient balance`

**Solution:** Fund your account using Friendbot:
```bash
curl "https://friendbot.stellar.org?addr=$(stellar keys address alice)"
```

### Frontend Can't Connect to Wallet

**Error:** `Failed to connect wallet`

**Solution:** 
1. Ensure you have Freighter wallet installed
2. Switch wallet to Testnet network
3. Check that `NEXT_PUBLIC_STELLAR_NETWORK=testnet` in `.env.local`

### Backend Can't Index Events

**Error:** `RPC connection timeout`

**Solution:** 
1. Verify RPC URL is correct in `.env`
2. Check Stellar testnet status: https://status.stellar.org
3. Try alternative RPC: `https://soroban-testnet.stellar.org:443`

### Contract Build Fails

**Error:** `wasm32-unknown-unknown target not found`

**Solution:** Add wasm target:
```bash
rustup target add wasm32-unknown-unknown
```

### Tests Failing

**Error:** `Network connection error`

**Solution:** Ensure contracts are deployed and environment variables are set correctly in test config.

---

## 📚 Documentation & Resources


- **Stellar Documentation:** [developers.stellar.org](https://developers.stellar.org/docs/build/smart-contracts)
- **Soroban Docs:** [soroban.stellar.org/docs](https://soroban.stellar.org/docs)
- **Off-Chain Oracle Architecture:** [contracts/README.md](./contracts/README.md)
- **Soroban Examples:** [github.com/stellar/soroban-examples](https://github.com/stellar/soroban-examples)

---

## 🤝 Contributing

We welcome contributions from the community! Here's how you can help:

### Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Follow the setup instructions above
4. Make your changes
5. Write/update tests
6. Ensure all tests pass (`npm test` in relevant directory)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to your branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Coding Standards

- **Rust (Contracts):** Follow Rust standard formatting (`cargo fmt`)
- **TypeScript (Frontend/Backend):** Use ESLint and Prettier configs provided
- **Commits:** Use conventional commits (e.g., `feat:`, `fix:`, `docs:`)
- **Tests:** Write tests for new features and bug fixes

### Pull Request Guidelines

- Provide clear description of changes
- Reference any related issues
- Ensure CI checks pass
- Update documentation if needed
- Add screenshots for UI changes

### Communication

- **GitHub Issues:** Bug reports and feature requests
- **GitHub Discussions:** Questions and general discussion
- **Discord:** [Join our Telegram](@h2YC0g3Xx_Y5OGZk)
- **Email:** dev@nestera.io _(if available)_

---

## 🗺 Roadmap

### Current Phase (Q1 2026)
- ✅ Core savings contract
- ✅ Basic web interface
- 🚧 Group savings pools
- 🚧 Interest calculation optimization

### Next Phase (Q2 2026)
- Goal-based savings UI
- Notification system
- Mobile app (Flutter)
- Mainnet deployment

### Future
- Cross-chain savings
- Yield strategies integration
- DAO governance
- Advanced analytics dashboard

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Stellar Development Foundation for Soroban platform
- Drips Wave for grants and support
- Open-source contributors and testers

---

## 📞 Support

Need help? Here's how to get support:

1. Check the [Troubleshooting](#-troubleshooting) section
2. Search [existing issues](https://github.com/your-org/nestera/issues)
3. Open a [new issue](https://github.com/your-org/nestera/issues/new) with detailed information
4. Join our [Discord community](https://discord.gg/nestera) _(if available)_

---

**Built with ❤️ on Stellar**
