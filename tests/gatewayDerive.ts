import * as anchor from "@project-serum/anchor";
import {AnchorProvider, Program, Wallet, web3} from "@project-serum/anchor";
import {GatewayDerive} from "../target/types/gateway_derive";
import {
  addGatekeeper,
  sendGatewayTransaction,
} from "../src/lib/util";
import chai from "chai";
import {fund} from "./gatekeeperUtils";
import {pluck} from 'ramda';
import {GatekeeperService} from "@identity.com/solana-gatekeeper-lib";
import chaiAsPromised from "chai-as-promised";

import {
  findGatewayToken,
} from "@identity.com/solana-gateway-ts";
import {DerivedPassService} from "../src/DerivedPassService";

chai.use(chaiAsPromised);

const { expect } = chai;

describe("gateway-derive", () => {
  const authorityProvider = AnchorProvider.env();
  anchor.setProvider(authorityProvider);
  const authority = authorityProvider.wallet.publicKey;

  const program = anchor.workspace.GatewayDerive as Program<GatewayDerive>;

  // gatekeeper networks (pass types) that are used to make up the derived pass
  // a wallet must have passes in each of these GKNs to be eligible for the derived pass
  const sourceGkns = [
    web3.Keypair.generate(),
    web3.Keypair.generate()
  ];
  const sourceGknKeys = pluck('publicKey', sourceGkns);

  // issues passes in the sourceGkns
  const civicGatekeeper = web3.Keypair.generate();

  let derivedPass: web3.PublicKey;
  let civicGatekeeperServices: GatekeeperService[];

  // the recipient of the derived pass
  // they should be able to self-serve generation of the derived pass
  // if they have the requisite component passes
  let owner: web3.Keypair;
  let ownerProvider: AnchorProvider;

  let service: DerivedPassService;

  before('Set up the component pass gatekeeper networks and add the gatekeeper to each', async () => {
    await fund(authorityProvider, civicGatekeeper.publicKey);

    civicGatekeeperServices = await Promise.all(sourceGkns.map(async gkn => {
      await fund(authorityProvider, gkn.publicKey)
      return addGatekeeper(authorityProvider, gkn, civicGatekeeper);
    }));
  });

  beforeEach('set up the owner (recipient) of the pass, and fund the authority', async () => {
    owner = web3.Keypair.generate();
    ownerProvider = new AnchorProvider(authorityProvider.connection, new Wallet(owner), AnchorProvider.defaultOptions());

    await fund(authorityProvider, owner.publicKey);
  });

  context("derived pass creation", () => {
    before(() => {
      service = new DerivedPassService(program, authorityProvider);
    });

    it("creates a new derived pass", async () => {
      const [, derivedPass] = await service.derivePass(sourceGknKeys);

      const derivedPassAccount = await program.account.derivedPass.fetch(derivedPass);
      expect(derivedPassAccount.sourceGkns).to.deep.equal(sourceGknKeys);
    });
  });

  context('derived pass issuance', () => {
    beforeEach('generate the derived pass', async () => {
      const authorityService = new DerivedPassService(program, authorityProvider);
      [, derivedPass] = await authorityService.derivePass(sourceGknKeys);

      service = new DerivedPassService(program, ownerProvider);
    })

    context("with no passes in the owner's wallet", ()=> {
      it("should not be able to derive a pass", async () => {
        const shouldFail = service.issue(authority, derivedPass);

        return expect(shouldFail).to.be.rejectedWith(/MissingComponentPass/);
      });
    });

    context("when the owner has the requisite component passes", ()=> {
      beforeEach('issue the component passes', async () => {
        await Promise.all(civicGatekeeperServices.map(gks =>
          sendGatewayTransaction(() => gks.issue(owner.publicKey))
        ));
      });

      it("should be able to derive a pass", async () => {
        const [, gatewayToken] = await service.issue(authority, derivedPass);

        const foundToken = await findGatewayToken(authorityProvider.connection, owner.publicKey, derivedPass);
        expect(foundToken.publicKey.toBase58()).to.equal(gatewayToken.toBase58());
      });
    });
  });
});
