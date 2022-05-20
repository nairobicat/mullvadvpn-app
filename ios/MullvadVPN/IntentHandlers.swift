//
//  IntentHandlers.swift
//  MullvadVPN
//
//  Created by pronebird on 26/05/2022.
//  Copyright © 2022 Mullvad VPN AB. All rights reserved.
//

import Foundation

final class StartVPNIntentHandler: NSObject, StartVPNIntentHandling {
    func handle(intent: StartVPNIntent, completion: @escaping (StartVPNIntentResponse) -> Void) {
        TunnelManager.shared.startTunnel { operationCompletion in
            let code: StartVPNIntentResponseCode = operationCompletion.isSuccess
                ? .success : .failure
            let response = StartVPNIntentResponse(code: code, userActivity: nil)

            completion(response)
        }
    }
}

final class StopVPNIntentHandler: NSObject, StopVPNIntentHandling {
    func handle(intent: StopVPNIntent, completion: @escaping (StopVPNIntentResponse) -> Void) {
        TunnelManager.shared.stopTunnel { operationCompletion in
            let code: StopVPNIntentResponseCode = operationCompletion.isSuccess
                ? .success : .failure
            let response = StopVPNIntentResponse(code: code, userActivity: nil)

            completion(response)
        }
    }
}

final class SelectNextVPNRelayIntentHandler: NSObject, SelectNextVPNRelayIntentHandling {
    func handle(intent: SelectNextVPNRelayIntent, completion: @escaping (SelectNextVPNRelayIntentResponse) -> Void) {
        TunnelManager.shared.reconnectTunnel { operationCompletion in
            let code: SelectNextVPNRelayIntentResponseCode = operationCompletion.isSuccess
                ? .success : .failure

            completion(SelectNextVPNRelayIntentResponse(code: code, userActivity: nil))
        }
    }
}
