<!-- <img src="https://github.com/MutinyWallet/harbor/assets/132156916/899ad35c-0bf9-4341-a0df-231ce81a9a65" width=25% height=25%> -->
<img src="https://blog.mutinywallet.com/content/images/size/w2000/2024/05/harbor-preview.jpeg" width=100% height=100%>

# Harbor

Harbor is an ecash desktop wallet for better bitcoin privacy. Use this tool to interact with ecash mints, moving money in and out using existing Bitcoin wallets. As you use mints, you may be able to increase the privacy of your money. Harbor also aims to demystify ecash mints for users and make them easier to use.

Highlights:
- Ecash - digital payments privacy technology
- Bitcoin - on-chain and lightning
- Privacy - everything runs over tor
- Multi-mint - spread funds over multiple mints
- Automation - can run in the background and move your funds automatically

**NOTE:** This is alpha software that could rapidly change in feature set. There is risk of losing funds. Compile and run at your own risk.

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

⚠️ NOTE: Nix OS support on linux environments is still in progress, this may or may not work for you yet: https://github.com/MutinyWallet/harbor/issues/7

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

If you're on linux you may need to exit the nix shell to be able to run the program.

```
// debug build
just run
```

```
// release build
just release
```
**NOTE**: The first password you type in the box is saved as your password. There will be a proper onboarding workflow in the future.

#### Database Changes
Reset local DB (for init, schema generation, etc.)

```
just reset-db
```

## Feedback & Support

A product like this is unique. We need all the feedback, help, and support we can get. We believe in building a tool like this out in the open as fully open-sourced MIT code that is freely available and does not depend on a centralized coordinator or single developer. However, we're unable to gain insights into how people use this tool, whether users like it or how many users even exist.

Therefore, we need your help. For one, we need feedback. Do you want to use a tool like this? What features are most important to you, and what do you want? Please use the discussion boards here on GitHub or the [Harbor channel on our Discord](https://discord.gg/5fFBKkcW). This will primarily drive the Harbor feature set.

Building free and open-source software is not free to us developers. While we believe in this tool's mission, we must rely on donations. We cannot profit from transactions for this service, and it must be fully open-sourced for this community to use it.

Visit our [Geyser funding page](https://geyser.fund/project/harbor). Any donations are greatly appreciated for funding development and signaling that it's a valuable tool to you.
