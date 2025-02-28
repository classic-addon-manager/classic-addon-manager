// @ts-check
// Cynhyrchwyd y ffeil hon yn awtomatig. PEIDIWCH Â MODIWL
// This file is automatically generated. DO NOT EDIT

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore: Unused imports
import {Call as $Call, Create as $Create} from "@wailsio/runtime";

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore: Unused imports
import * as addon$0 from "../addon/models.js";

/**
 * @returns {Promise<addon$0.AddonManifest[]> & { cancel(): void }}
 */
export function GetAddonManifest() {
    let $resultPromise = /** @type {any} */($Call.ByID(1299347432));
    let $typingPromise = /** @type {any} */($resultPromise.then(($result) => {
        return $$createType1($result);
    }));
    $typingPromise.cancel = $resultPromise.cancel.bind($resultPromise);
    return $typingPromise;
}

/**
 * @param {addon$0.AddonManifest} ad
 * @returns {Promise<boolean> & { cancel(): void }}
 */
export function InstallAddon(ad) {
    let $resultPromise = /** @type {any} */($Call.ByID(699642360, ad));
    return $resultPromise;
}

// Private type creation functions
const $$createType0 = addon$0.AddonManifest.createFrom;
const $$createType1 = $Create.Array($$createType0);
