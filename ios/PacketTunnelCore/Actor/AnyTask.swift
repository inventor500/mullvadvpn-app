//
//  AnyTask.swift
//  PacketTunnel
//
//  Created by pronebird on 28/08/2023.
//  Copyright © 2025 Mullvad VPN AB. All rights reserved.
//

import Foundation

/// A type-erased `Task`.
public protocol AnyTask: Sendable {
    /// Cancel task.
    func cancel()
}

extension Task: AnyTask {}
