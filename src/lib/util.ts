import {AnchorProvider, Program, Provider, web3, Wallet} from '@project-serum/anchor';
import {findGatewayToken, getGatekeeperAccountAddress} from '@identity.com/solana-gateway-ts'
import {GatewayDerive} from '../../target/types/gateway_derive';
import {
  GatekeeperNetworkService,
  GatekeeperService,
  SendableDataTransaction
} from "@identity.com/solana-gatekeeper-lib";

const GATEKEEPER_SEED = 'gateway_derive_gk_seed';
const DERIVED_PASS_PROGRAM = new web3.PublicKey('dpKGstEdwqh8pDfFh3Qrp1yJ85xbvbZtTcjRaq1yqip');

export const deriveGatekeeper = async (authority: web3.PublicKey, program: Program<GatewayDerive>) =>
  web3.PublicKey.findProgramAddress(
    [Buffer.from(GATEKEEPER_SEED), authority.toBuffer()],
    program.programId
  );

export const deriveGatekeeperAccount = async (gatekeeper: web3.PublicKey, authority: web3.PublicKey) =>
  getGatekeeperAccountAddress(gatekeeper, authority);


export const fetchProgram = async (provider: Provider): Promise<Program<GatewayDerive>> => {
  const idl = await Program.fetchIdl<GatewayDerive>(DERIVED_PASS_PROGRAM, provider);

  if (!idl) throw new Error('Notification IDL could not be found');

  return new Program<GatewayDerive>(idl, DERIVED_PASS_PROGRAM, provider) as Program<GatewayDerive>;
};

export const sendGatewayTransaction = <T>(fn: () => Promise<SendableDataTransaction<T | null>>) =>
  fn()
    .then((result) => result.send())
    .then(async (sendResult) => {
      const resultData = await sendResult.confirm();
      if (!resultData) throw new Error('Failed to execute transaction');
      return resultData;
    });

export const addGatekeeper = async (provider: AnchorProvider, gatekeeperNetwork: web3.Keypair, gatekeeper: web3.Keypair): Promise<GatekeeperService> => {
  // create a new gatekeeper network (no on-chain tx here)
  const gknService = new GatekeeperNetworkService(provider.connection, gatekeeperNetwork);
  const gkService = new GatekeeperService(provider.connection, gatekeeperNetwork.publicKey, gatekeeper);

  // add the civic gatekeeper to this network
  await sendGatewayTransaction(() => gknService.addGatekeeper(gatekeeper.publicKey));

  return gkService;
}

/**
 * Convert a public key into an accountMeta object for passing into an instruction.
 * Assumes the account is not writeable or a signer
 * @param publicKey
 */
export const toSimpleAccountMeta = (publicKey: web3.PublicKey): web3.AccountMeta => ({
  pubkey: publicKey,
  isSigner: false,
  isWritable: false,
})

export const calculateDerivedPassSize = (sourceGkns: web3.PublicKey[]) => 16 + (sourceGkns.length * 32) + 32

export const findComponentPassesForDerivedPass = async (program: Program<GatewayDerive>, derivedPass: web3.PublicKey, owner: web3.PublicKey):Promise<web3.PublicKey[]> => {
  const derivedPassAccount = await program.account.derivedPass.fetch(derivedPass);
  const sourcePassTypes = derivedPassAccount.sourceGkns;
  const sourcePassPromises = sourcePassTypes.map((sourcePassType) => findGatewayToken(program.provider.connection, owner, sourcePassType));
  const sourcePasses = await Promise.all(sourcePassPromises);
  return sourcePasses
    .filter(Boolean)
    .map((sourcePass) => sourcePass.publicKey);
}
