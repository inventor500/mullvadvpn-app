//
//  DeviceCheckRemoteServiceProtocol.swift
//  PacketTunnel
//
//  Created by pronebird on 07/06/2023.
//  Copyright © 2025 Mullvad VPN AB. All rights reserved.
//

import Foundation
import MullvadTypes
import WireGuardKitTypes

/// A protocol that formalizes remote service dependency used by `DeviceCheckOperation`.
protocol DeviceCheckRemoteServiceProtocol {
    func getAccountData(accountNumber: String, completion: @escaping @Sendable (Result<Account, Error>) -> Void)
        -> Cancellable
    func getDevice(
        accountNumber: String,
        identifier: String,
        completion: @escaping @Sendable (Result<Device, Error>) -> Void
    )
        -> Cancellable
    func rotateDeviceKey(
        accountNumber: String,
        identifier: String,
        publicKey: PublicKey,
        completion: @escaping @Sendable (Result<Device, Error>) -> Void
    ) -> Cancellable
}
