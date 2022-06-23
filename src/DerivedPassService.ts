import { AnchorProvider, Program, web3 } from "@project-serum/anchor";
import {
  calculateDerivedPassSize,
  deriveGatekeeper,
  deriveGatekeeperAccount,
  deriveGatekeeperFeeAddress,
  fetchProgram,
  findComponentPassesForDerivedPass,
  toAccountMeta,
  toSimpleAccountMeta,
} from "./lib/util";
import { GatewayDerive } from "../target/types/gateway_derive";
import * as anchor from "@project-serum/anchor";
import {
  getGatewayTokenAddressForOwnerAndGatekeeperNetwork,
  PROGRAM_ID as GATEWAY_PROGRAM_ID,
} from "@identity.com/solana-gateway-ts";
import { PublicKey, Transaction } from "@solana/web3.js";

// TODO remove once Anchor cleans up its Wallet interface
/**
 * https://github.com/project-serum/anchor/blob/master/ts/src/index.ts#L21
 * Wallet implements NodeWallet, NodeWallet implements Wallet
 * https://github.com/project-serum/anchor/blob/master/ts/src/nodewallet.ts#L8
 * We only want the generic wallet interface, not the Node one
 */
export interface Wallet {
  signTransaction(tx: Transaction): Promise<Transaction>;
  signAllTransactions(txs: Transaction[]): Promise<Transaction[]>;
  publicKey: PublicKey;
}

export class DerivedPassService {
  private program: Program<GatewayDerive>;

  static async build(provider: AnchorProvider): Promise<DerivedPassService> {
    const program = await fetchProgram(provider);
    return new DerivedPassService(program, provider);
  }

  constructor(
    program: Program<GatewayDerive>,
    private provider: AnchorProvider
  ) {
    this.program = new Program<GatewayDerive>(
      program.idl,
      program.programId,
      provider,
      program.coder
    );
  }

  async derivePass(
    sourcePassTypes: web3.PublicKey[]
  ): Promise<[string, web3.PublicKey]> {
    const derivedPass = web3.Keypair.generate();
    const authority = this.provider.wallet.publicKey;

    const [derivedGatekeeper, derivedGatekeeperBump] = await deriveGatekeeper(
      authority,
      this.program
    );
    const derivedGatekeeperAccount = await deriveGatekeeperAccount(
      derivedGatekeeper,
      derivedPass.publicKey
    );

    const accounts = {
      derivedPass: derivedPass.publicKey,
      authority,
      derivedGatekeeper,
      derivedGatekeeperAccount,
      gatewayProgram: GATEWAY_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    };

    const transactionSignature = await this.program.methods
      .initialize(
        sourcePassTypes,
        calculateDerivedPassSize(sourcePassTypes),
        derivedGatekeeperBump
      )
      .accounts(accounts)
      .signers([derivedPass])
      .rpc();

    await this.provider.connection.confirmTransaction(transactionSignature);

    return [transactionSignature, derivedPass.publicKey];
  }

  async issue(
    authority: web3.PublicKey,
    derivedPass: web3.PublicKey
  ): Promise<[string, web3.PublicKey]> {
    const recipient = this.provider.wallet.publicKey;
    const gatewayToken =
      await getGatewayTokenAddressForOwnerAndGatekeeperNetwork(
        recipient,
        derivedPass
      );
    const [derivedGatekeeper] = await deriveGatekeeper(authority, this.program);
    const derivedGatekeeperAccount = await deriveGatekeeperAccount(
      derivedGatekeeper,
      derivedPass
    );

    const componentPasses = await findComponentPassesForDerivedPass(
      this.program,
      derivedPass,
      recipient
    );
    const feeAdddressPromises = componentPasses.map((pass) =>
      deriveGatekeeperFeeAddress(
        pass.issuingGatekeeper,
        pass.gatekeeperNetwork,
        this.program
      )
    );

    const feeAddressesAndBumps = await Promise.all(feeAdddressPromises);

    const componentPassAccounts = componentPasses
      .map((pass) => pass.publicKey)
      .map(toSimpleAccountMeta);
    const feeAddressAccounts = feeAddressesAndBumps
      .map(([key]) => key)
      .map(toSimpleAccountMeta);
    // pass the fee address derivation bumps so that the derivation can be checked on the program
    const feeAddressBumps = feeAddressesAndBumps.map(([, bump]) => bump);
    // pass the gatekeeper accounts as writeable so that they can receive payment
    const gatekeeperAccounts = componentPasses
      .map((pass) => pass.issuingGatekeeper)
      .map(toAccountMeta(false, true));

    const accounts = {
      derivedPass,
      recipient,
      derivedGatekeeper,
      derivedGatekeeperAccount,
      gatewayToken,
      gatewayProgram: GATEWAY_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    };

    // Each component pass is associated with a fee account (it may be empty) and a gatekeeper account (to receive payment)
    // Note: The gatekeepers may be duplicated here, if the same gatekeeper issues more than one component pass.
    // This is handled in the program.
    const remainingAccounts = [
      ...componentPassAccounts,
      ...feeAddressAccounts,
      ...gatekeeperAccounts,
    ];
    const transactionSignature = await this.program.methods
      .issue(Buffer.from(feeAddressBumps))
      .accounts(accounts)
      .remainingAccounts(remainingAccounts)
      .rpc();

    await this.provider.connection.confirmTransaction(transactionSignature);

    return [transactionSignature, gatewayToken];
  }

  async setFee(
    gatekeeperNetwork: web3.PublicKey,
    fee: number
  ): Promise<string> {
    const [feeAddress, bump] = await deriveGatekeeperFeeAddress(
      this.provider.wallet.publicKey,
      gatekeeperNetwork,
      this.program
    );

    const accounts = {
      fee: feeAddress,
      authority: this.provider.wallet.publicKey,
      gatekeeperNetwork,
      systemProgram: anchor.web3.SystemProgram.programId,
    };

    console.log("accounts", {
      fee: feeAddress.toBase58(),
      authority: this.provider.wallet.publicKey.toBase58(),
      gatekeeperNetwork: gatekeeperNetwork.toBase58(),
    });

    const feeAlreadyExists = await this.provider.connection
      .getAccountInfo(feeAddress)
      .then((info) => info && info.owner.equals(this.program.programId));

    const callSetFee = feeAlreadyExists
      ? this.program.methods.updateFee
      : this.program.methods.createFee;

    const transactionSignature = await callSetFee(new anchor.BN(fee), null)
      .accounts(accounts)
      .rpc();

    await this.provider.connection.confirmTransaction(transactionSignature);

    return transactionSignature;
  }

  async unsetFee(gatekeeperNetwork: web3.PublicKey): Promise<string> {
    const [feeAddress, bump] = await deriveGatekeeperFeeAddress(
      this.provider.wallet.publicKey,
      gatekeeperNetwork,
      this.program
    );

    const accounts = {
      fee: feeAddress,
      authority: this.provider.wallet.publicKey,
      gatekeeperNetwork,
    };

    console.log("accounts", {
      fee: feeAddress.toBase58(),
      authority: this.provider.wallet.publicKey.toBase58(),
      gatekeeperNetwork: gatekeeperNetwork.toBase58(),
    });

    const transactionSignature = await this.program.methods
      .removeFee()
      .accounts(accounts)
      .rpc();

    await this.provider.connection.confirmTransaction(transactionSignature);

    return transactionSignature;
  }
}
