<p align="center">
  <a href="https://wvm.dev">
    <img src="https://raw.githubusercontent.com/weaveVM/.github/main/profile/bg.png">
  </a>
</p>

## Synopsis 
Operators-As-ExEx is a paradigm that integrates partially or completely the operator of an Actively Validated Services as [Execution Extensions](https://exex.rs) in Reth. This approach aligns perfectly with events-driven activated predefined-actions (EDAs) such as Keeper Networks.

## About This ExEx
This ExEx is the re-implementation of Eigenlayer's [hello-world-avs](https://github.com/Layr-Labs/hello-world-avs) as an ExEx. this work comes to demonstrate in practice how the Operator-As-ExEx paradigm works.

## Operators-As-ExEx Workflow
![](./assets/workflow.png)

## Run it

First of all, make sure to setup your `.env` file according to [.env.example](./.env.example) 

```bash
git clone https://github.com/weaveVM/hello-world-avs-as-exex.git

cd hello-world-aws-as-exex

cargo build

cargo run -- init --chain holesky --datadir data

cargo run node
```

## License
This project is licensed under the [MIT License](./LICENSE)