import * as anchor from "@project-serum/anchor";
import { AnchorProvider, Program, Wallet, web3 } from "@project-serum/anchor";
import chai from "chai";
import chaiAsPromised from "chai-as-promised";
import sinon from "sinon";
import { pluck } from "ramda";
import { GatekeeperService } from "@identity.com/solana-gatekeeper-lib";
import { findGatewayToken } from "@identity.com/solana-gateway-ts";

import { GatewayDerive } from "../target/types/gateway_derive";
import { addGatekeeper, fund, sendGatewayTransaction } from "./gatekeeperUtils";

import { DerivedPassService } from "../src/";
import * as util from "../src/lib/util";
import { deriveGatekeeperFeeAddress } from "../src/lib/util";

chai.use(chaiAsPromised);

const { expect } = chai;

const sandbox = sinon.createSandbox();

describe("gateway-derive", () => {
  afterEach(sandbox.restore);
  const authorityProvider = AnchorProvider.env();
  anchor.setProvider(authorityProvider);
  const authority = authorityProvider.wallet.publicKey;

  const program = anchor.workspace.GatewayDerive as Program<GatewayDerive>;

  // gatekeeper networks (pass types) that are used to make up the derived pass
  // a wallet must have passes in each of these GKNs to be eligible for the derived pass
  const sourceGkns = [web3.Keypair.generate(), web3.Keypair.generate()];
  const sourceGknKeys = pluck("publicKey", sourceGkns);

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

  before(
    "Set up the component pass gatekeeper networks and add the gatekeeper to each",
    async () => {
      if (process.env.ENABLE_LOGS)
        authorityProvider.connection.onLogs("all", (log) =>
          console.log(log.logs)
        );

      await fund(authorityProvider, civicGatekeeper.publicKey);

      civicGatekeeperServices = await Promise.all(
        sourceGkns.map(async (gkn) => {
          await fund(authorityProvider, gkn.publicKey);
          return addGatekeeper(authorityProvider, gkn, civicGatekeeper);
        })
      );
    }
  );

  beforeEach(
    "set up the owner (recipient) of the pass, and fund the authority",
    async () => {
      owner = web3.Keypair.generate();
      ownerProvider = new AnchorProvider(
        authorityProvider.connection,
        new Wallet(owner),
        AnchorProvider.defaultOptions()
      );

      await fund(authorityProvider, owner.publicKey);
    }
  );

  context("derived pass creation", () => {
    before(() => {
      service = new DerivedPassService(program, authorityProvider);
    });

    it("creates a new derived pass", async () => {
      const [, derivedPass] = await service.derivePass(sourceGknKeys);

      const derivedPassAccount = await program.account.derivedPass.fetch(
        derivedPass
      );
      expect(derivedPassAccount.sourceGkns).to.deep.equal(sourceGknKeys);
    });
  });

  context("derived pass issuance", () => {
    beforeEach("generate the derived pass", async () => {
      const authorityService = new DerivedPassService(
        program,
        authorityProvider
      );
      [, derivedPass] = await authorityService.derivePass(sourceGknKeys);

      service = new DerivedPassService(program, ownerProvider);
    });

    context("with no passes in the owner's wallet", () => {
      it("should not be able to derive a pass", async () => {
        const shouldFail = service.issue(authority, derivedPass);

        return expect(shouldFail).to.be.rejectedWith(/MissingComponentPass/);
      });
    });

    context("when the owner has the requisite component passes", () => {
      beforeEach("issue the component passes", async () => {
        await Promise.all(
          civicGatekeeperServices.map((gks) =>
            sendGatewayTransaction(() => gks.issue(owner.publicKey))
          )
        );
      });

      it("should be able to derive a pass", async () => {
        const [, gatewayToken] = await service.issue(authority, derivedPass);

        const foundToken = await findGatewayToken(
          authorityProvider.connection,
          owner.publicKey,
          derivedPass
        );
        expect(foundToken.publicKey.toBase58()).to.equal(
          gatewayToken.toBase58()
        );
      });

      context("with fees", () => {
        // fees for the two constituent passes
        const fee0 = 100;
        const fee1 = 1000;

        let civicGatekeeperDerivedPassService: DerivedPassService;

        before(() => {
          const civicGatekeeperProvider = new AnchorProvider(
            authorityProvider.connection,
            new Wallet(civicGatekeeper),
            AnchorProvider.defaultOptions()
          );
          civicGatekeeperDerivedPassService = new DerivedPassService(
            program,
            civicGatekeeperProvider
          );
        });

        it("should register a fee", async () => {
          await civicGatekeeperDerivedPassService.setFee(
            sourceGkns[0].publicKey,
            fee0
          );
        });

        // Warning, this relies on the previous test running first to set the fee
        it("should pay the gatekeeper for one of the component passes", async () => {
          const previousGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          await service.issue(authority, derivedPass);

          const newGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          expect(newGatekeeperBalance - previousGatekeeperBalance).to.equal(
            fee0
          );
        });

        // Warning, this relies on the first test in the suite running first to set the fee
        it("should pay the gatekeeper for both of the component passes", async () => {
          // set the second fee
          await civicGatekeeperDerivedPassService.setFee(
            sourceGkns[1].publicKey,
            fee1
          );

          const previousGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          await service.issue(authority, derivedPass);

          const newGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          expect(newGatekeeperBalance - previousGatekeeperBalance).to.equal(
            fee0 + fee1
          );
        });

        it("should update the fee", async () => {
          // double the second fee
          await civicGatekeeperDerivedPassService.setFee(
            sourceGkns[1].publicKey,
            2 * fee1
          );

          const previousGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          await service.issue(authority, derivedPass);

          const newGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          const expectedTotalFee = fee0 + 2 * fee1;
          expect(newGatekeeperBalance - previousGatekeeperBalance).to.equal(
            expectedTotalFee
          );
        });

        it("should fail if an invalid fee account is passed", () => {
          sandbox
            .stub(util, "deriveGatekeeperFeeAddress")
            .resolves([web3.Keypair.generate().publicKey, 255]);

          const shouldFail = service.issue(authority, derivedPass);

          return expect(shouldFail).to.be.rejectedWith(/InvalidFeeAccount/);
        });

        it("should fail if an incorrect gatekeeper account is passed", () => {
          // stub toAccountMeta to pass an incorrect gatekeeper account for the gateekeeper
          sandbox
            .stub(util, "toAccountMeta")
            .callsFake((isSigner, isWritable) => (pubkey: web3.PublicKey) => {
              if (pubkey.equals(civicGatekeeper.publicKey))
                return {
                  pubkey: web3.Keypair.generate().publicKey, // dummy gatekeeper
                  isSigner,
                  isWritable,
                };

              return {
                pubkey,
                isSigner,
                isWritable,
              };
            });

          const shouldFail = service.issue(authority, derivedPass);

          return expect(shouldFail).to.be.rejectedWith(/GatekeeperMismatch/);
        });

        it("should fail to remove the fee if not the owner", async () => {
          // create a different gatekeeper that will attempt to remove the fee
          const differentGatekeeper = web3.Keypair.generate();
          const differentGatekeeperProvider = new AnchorProvider(
            authorityProvider.connection,
            new Wallet(differentGatekeeper),
            AnchorProvider.defaultOptions()
          );
          const differentGatekeeperDerivedPassService = new DerivedPassService(
            program,
            differentGatekeeperProvider
          );

          // stub the client to pass the civic gatekeeper fee address
          const deriveResult = await deriveGatekeeperFeeAddress(
            civicGatekeeper.publicKey,
            sourceGkns[1].publicKey,
            program
          );
          sandbox
            .stub(util, "deriveGatekeeperFeeAddress")
            .resolves(deriveResult);

          await fund(authorityProvider, differentGatekeeper.publicKey);

          const shouldFail = differentGatekeeperDerivedPassService.unsetFee(
            sourceGkns[1].publicKey
          );

          // The derived fee account address will not match the passed-in seeds
          return expect(shouldFail).to.be.rejectedWith(/ConstraintSeeds/);
        });

        it("should remove the fee", async () => {
          await civicGatekeeperDerivedPassService.unsetFee(
            sourceGkns[1].publicKey
          );

          await new Promise((resolve) => setTimeout(resolve, 5000));

          const [feeAddress] = await deriveGatekeeperFeeAddress(
            civicGatekeeper.publicKey,
            sourceGkns[1].publicKey,
            program
          );

          const account = await authorityProvider.connection.getAccountInfo(
            feeAddress
          );

          expect(account).to.be.null;

          const previousGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          await service.issue(authority, derivedPass);

          const newGatekeeperBalance =
            await authorityProvider.connection.getBalance(
              civicGatekeeper.publicKey
            );

          // no fee1 incurred this time.
          const expectedTotalFee = fee0;
          expect(newGatekeeperBalance - previousGatekeeperBalance).to.equal(
            expectedTotalFee
          );
        });
      });
    });
  });
});
