Smallet Repository 

Solana Account Abstraction / Timelock Wallet Implementation 

-----------------------BUILD----------------------

To build follow these instructions:

First run:

```
yarn instsll
```

Then run:

```
cargo check && cargo build
```

Next run:

```
./scripts/parse-idls.sh && ./scripts/generate-idl-types.sh
```

Then run:

```
rm -fr dist/ && node_modules/.bin/tsc -P tsconfig.build.json && node_modules/.bin/tsc -P tsconfig.esm.json
```

-----------------------TEST-------------------------

To run tests, run this command in your terminal:

```
anchor test --skip-build tests/*.spec.ts
```

or using mocha, run:

```
yarn mocha -b
```

