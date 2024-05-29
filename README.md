<img src="https://github.com/MutinyWallet/harbor/assets/132156916/899ad35c-0bf9-4341-a0df-231ce81a9a65" width=25% height=25%>

# Harbor

Harbor is an ecash desktop wallet for better bitcoin privacy. Use this tool to interact with ecash mints, moving money in and out using existing Bitcoin wallets. As you use mints, you may be able to increase the privacy of your money. Harbor also aims to demystify ecash mints for users and make them easier to use.

Highlights:
- Ecash - digital payments privacy technology
- Bitcoin - on-chain and lightning
- Privacy - everything runs over tor
- Multi-mint - spread funds over multiple mints
- Automation - can run in the background and move your funds automatically

**NOTE:** This is brand new alpha software that could rapidly change in feature set. There is risk of losing funds. Compile and run at your own risk.

### Compatibility
Harbor is a desktop app built in Rust, using the [iced](https://iced.rs) framework, that runs on Mac, Windows, and Linux. It currently supports [Fedimint](https://fedimint.org), Bitcoin, and Lightning. (We see you, Cashu 👀)

Binaries will be available in the future. For now you need to compile it yourself following the instructions below.

<img src="https://harbor.cash/screens/home.png" width=50% height=50%>

## Building

1. Clone the `MutinyWallet/harbor` repo and `cd` into it.

```
git clone <harbor git URL> harbor
cd harbor
```

2. Install NixOS on your machine if you do not have it already.

```
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
// Follow any Nix installation instructions in the terminal, including post install steps.
```

3. Everything is done in a nix develop shell for now: 

```
nix develop
```

4. Run the unit tests
```
just test
```

5. Build and Run

```
// debug build
just run
```

```
// release build
just release
```

#### Database Changes
Reset local DB (for init, schema generation, etc.)

```
just reset-db
```
