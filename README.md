# Harbor

Harbor is an ecash desktop wallet for better bitcoin privacy. Use this tool to interact with ecash mints, moving money in and out using existing Bitcoin wallets. As you use mints, you may be able to increase the privacy of your money. Harbor also aims to demystify ecash mints for users and make them easier to use.

Highlights:
- Ecash
- Bitcoin
- Privacy
- Multi-mint
- Automation

**NOTE:** This is brand new alpha software that could rapidly change in feature set. There is risk of losing funds. Compile and run at your own risk.

## Develop

1. Install nixos on your machine if you do not have it already: 

```
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

2. Everything is done in a nix develop shell for now: 

```
nix develop
```

3. Build, test, run, etc. 


```
just test
```

```
just run
```

```
just release
```

4. Reset local DB (for init, schema generation, etc.)

```
just reset-db
```
