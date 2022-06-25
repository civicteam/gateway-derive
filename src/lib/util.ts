import { Program, Provider, web3 } from "@project-serum/anchor";
import {
  findGatewayToken,
  GatewayToken,
  getGatekeeperAccountAddress,
} from "@identity.com/solana-gateway-ts";
import { GatewayDerive } from "../../target/types/gateway_derive";

const GATEKEEPER_SEED = "gateway_derive_gk_seed";
const FEE_SEED = "gateway_derive_fee_seed";
const DERIVED_PASS_PROGRAM = new web3.PublicKey(
  "dpKGstEdwqh8pDfFh3Qrp1yJ85xbvbZtTcjRaq1yqip"
);
const DERIVED_PASS_PROPERTIES_SIZE = 8 + 1 + 1;

export const deriveGatekeeper = async (
  authority: web3.PublicKey,
  program: Program<GatewayDerive>
): Promise<[web3.PublicKey, number]> =>
  web3.PublicKey.findProgramAddress(
    [Buffer.from(GATEKEEPER_SEED), authority.toBuffer()],
    program.programId
  );

export const deriveGatekeeperAccount = async (
  gatekeeper: web3.PublicKey,
  authority: web3.PublicKey
): Promise<web3.PublicKey> =>
  getGatekeeperAccountAddress(gatekeeper, authority);

export const deriveGatekeeperFeeAddress = async (
  gatekeeper: web3.PublicKey,
  gatekeeperNetwork: web3.PublicKey,
  program: Program<GatewayDerive>
): Promise<[web3.PublicKey, number]> =>
  web3.PublicKey.findProgramAddress(
    [
      Buffer.from(FEE_SEED),
      gatekeeper.toBuffer(),
      gatekeeperNetwork.toBuffer(),
    ],
    program.programId
  );

export const fetchProgram = async (
  provider: Provider
): Promise<Program<GatewayDerive>> => {
  const idl = await Program.fetchIdl<GatewayDerive>(
    DERIVED_PASS_PROGRAM,
    provider
  );

  if (!idl) throw new Error("Notification IDL could not be found");

  return new Program<GatewayDerive>(
    idl,
    DERIVED_PASS_PROGRAM,
    provider
  ) as Program<GatewayDerive>;
};

/**
 * Convert a public key into an accountMeta object for passing into an instruction.
 */
export const toAccountMeta =
  (isSigner: boolean, isWritable: boolean) =>
  (publicKey: web3.PublicKey): web3.AccountMeta => ({
    pubkey: publicKey,
    isSigner,
    isWritable,
  });

/**
 * Convert a public key into an accountMeta object for passing into an instruction.
 * Assumes the account is not writeable or a signer
 * @param publicKey
 */
export const toSimpleAccountMeta = toAccountMeta(false, false);

export const calculateDerivedPassSize = (sourceGkns: web3.PublicKey[]) =>
  16 + sourceGkns.length * 32 + 32 + DERIVED_PASS_PROPERTIES_SIZE;

export const findComponentPassesForDerivedPass = async (
  program: Program<GatewayDerive>,
  derivedPass: web3.PublicKey,
  owner: web3.PublicKey
): Promise<GatewayToken[]> => {
  const derivedPassAccount = await program.account.derivedPass.fetch(
    derivedPass
  );
  const sourcePassTypes = derivedPassAccount.sourceGkns;
  const sourcePassPromises = sourcePassTypes.map((sourcePassType) =>
    findGatewayToken(program.provider.connection, owner, sourcePassType)
  );
  const sourcePasses = await Promise.all(sourcePassPromises);
  return sourcePasses.filter(Boolean);
};

// should match the FeeType enum in lib.rs
// TODO Can anchor generate this mapping?
export type FeeType = "IssuerOnly";
export const feeTypeToInt = (feeType?: FeeType): number => {
  switch (feeType) {
    case "IssuerOnly":
      return 0;
    default:
      throw new Error(`Unknown strategy: ${feeType}`);
  }
};
