import {AnchorProvider, Program, web3} from "@project-serum/anchor";
import {
  calculateDerivedPassSize,
  deriveGatekeeper,
  deriveGatekeeperAccount,
  fetchProgram, findComponentPassesForDerivedPass,
  toSimpleAccountMeta
} from "./lib/util";
import { GatewayDerive } from '../target/types/gateway_derive';
import * as anchor from "@project-serum/anchor";
import {
  getGatewayTokenAddressForOwnerAndGatekeeperNetwork,
  PROGRAM_ID as GATEWAY_PROGRAM_ID
} from "@identity.com/solana-gateway-ts";
import {PublicKey, Transaction} from "@solana/web3.js";

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

  static async build(
    provider: AnchorProvider,
  ): Promise<DerivedPassService> {
    const program = await fetchProgram(provider);
    return new DerivedPassService(program, provider);
  }

  constructor(
    program: Program<GatewayDerive>,
    private provider: AnchorProvider,
  ){
    this.program = new Program<GatewayDerive>(program.idl, program.programId, provider, program.coder);
  }

  async derivePass(sourcePassTypes: web3.PublicKey[]): Promise<[string, web3.PublicKey]> {
    const derivedPass = web3.Keypair.generate();
    const authority = this.provider.wallet.publicKey;

    const [derivedGatekeeper, derivedGatekeeperBump] = await deriveGatekeeper(authority, this.program);
    const derivedGatekeeperAccount = await deriveGatekeeperAccount(derivedGatekeeper, derivedPass.publicKey);

    const accounts = {
      derivedPass: derivedPass.publicKey,
      authority,
      derivedGatekeeper,
      derivedGatekeeperAccount,
      gatewayProgram: GATEWAY_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    }

    const transactionSignature = await this.program.methods
      .initialize(sourcePassTypes, calculateDerivedPassSize(sourcePassTypes), derivedGatekeeperBump)
      .accounts(accounts)
      .signers([derivedPass]).rpc();

    await this.provider.connection.confirmTransaction(transactionSignature);

    return [transactionSignature, derivedPass.publicKey];
  }

  async issue(authority: web3.PublicKey, derivedPass: web3.PublicKey): Promise<[string, web3.PublicKey]> {
    const recipient = this.provider.wallet.publicKey;
    const gatewayToken = await getGatewayTokenAddressForOwnerAndGatekeeperNetwork(recipient, derivedPass);
    const [derivedGatekeeper] = await deriveGatekeeper(authority, this.program);
    const derivedGatekeeperAccount = await deriveGatekeeperAccount(derivedGatekeeper, derivedPass);

    const componentPasses = await findComponentPassesForDerivedPass(this.program, derivedPass, recipient);

    const accounts = {
      derivedPass,
      recipient,
      derivedGatekeeper,
      derivedGatekeeperAccount,
      gatewayToken,
      gatewayProgram: GATEWAY_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    };
    const transactionSignature = await this.program.methods.issue()
      .accounts(accounts)
      .remainingAccounts(componentPasses.map(toSimpleAccountMeta))
      .rpc();

    await this.provider.connection.confirmTransaction(transactionSignature);

    return [transactionSignature, gatewayToken];
  }
}
