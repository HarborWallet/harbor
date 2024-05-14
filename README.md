# Harbor

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
