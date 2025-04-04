//
//  TunnelInteractor.swift
//  MullvadVPN
//
//  Created by pronebird on 05/07/2022.
//  Copyright © 2025 Mullvad VPN AB. All rights reserved.
//

import Foundation
import MullvadREST
import MullvadSettings
import MullvadTypes
import PacketTunnelCore

protocol TunnelInteractor {
    // MARK: - Tunnel manipulation

    var tunnel: (any TunnelProtocol)? { get }
    var backgroundTaskProvider: BackgroundTaskProviding { get }

    func getPersistentTunnels() -> [any TunnelProtocol]
    func createNewTunnel() -> any TunnelProtocol
    func setTunnel(_ tunnel: (any TunnelProtocol)?, shouldRefreshTunnelState: Bool)

    // MARK: - Tunnel status

    var tunnelStatus: TunnelStatus { get }
    @discardableResult func updateTunnelStatus(_ block: @Sendable (inout TunnelStatus) -> Void) -> TunnelStatus

    // MARK: - Configuration

    var isConfigurationLoaded: Bool { get }
    var settings: LatestTunnelSettings { get }
    var deviceState: DeviceState { get }

    func setConfigurationLoaded()
    func setSettings(_ settings: LatestTunnelSettings, persist: Bool)
    func setDeviceState(_ deviceState: DeviceState, persist: Bool)
    func removeLastUsedAccount()
    func handleRestError(_ error: Error)

    func startTunnel()
    func prepareForVPNConfigurationDeletion()
    func selectRelays() throws -> SelectedRelays
}
