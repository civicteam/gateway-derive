import { AnchorProvider, web3 } from "@project-serum/anchor";
import {
  GatekeeperNetworkService,
  GatekeeperService,
  SendableDataTransaction,
} from "@identity.com/solana-gatekeeper-lib";

export const fund = (
  provider: AnchorProvider,
  to: web3.PublicKey,
  lamports: number = 500_000_000
) =>
  provider.sendAndConfirm(
    new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: to,
        lamports,
      })
    ),
    [],
    { commitment: "confirmed" }
  );

export const sendGatewayTransaction = <T>(
  fn: () => Promise<SendableDataTransaction<T | null>>
) =>
  fn()
    .then((result) => result.send())
    .then(async (sendResult) => {
      const resultData = await sendResult.confirm();
      if (!resultData) throw new Error("Failed to execute transaction");
      return resultData;
    });

export const addGatekeeper = async (
  provider: AnchorProvider,
  gatekeeperNetwork: web3.Keypair,
  gatekeeper: web3.Keypair
): Promise<GatekeeperService> => {
  // create a new gatekeeper network (no on-chain tx here)
  const gknService = new GatekeeperNetworkService(
    provider.connection,
    gatekeeperNetwork
  );
  const gkService = new GatekeeperService(
    provider.connection,
    gatekeeperNetwork.publicKey,
    gatekeeper
  );

  // add the civic gatekeeper to this network
  await sendGatewayTransaction(() =>
    gknService.addGatekeeper(gatekeeper.publicKey)
  );

  return gkService;
};
