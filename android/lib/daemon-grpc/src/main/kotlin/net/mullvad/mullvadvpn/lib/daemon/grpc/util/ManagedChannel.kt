package net.mullvad.mullvadvpn.lib.daemon.grpc.util

import co.touchlab.kermit.Logger
import io.grpc.ConnectivityState
import io.grpc.ManagedChannel
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.suspendCancellableCoroutine

@OptIn(ExperimentalCoroutinesApi::class)
internal fun ManagedChannel.connectivityFlow(): Flow<ConnectivityState> {
    return callbackFlow {
        var currentState = getState(false)

        while (isActive) {
            // Check that we are active before sending
            send(currentState)
            currentState = suspendCancellableCoroutine {
                notifyWhenStateChanged(currentState) {
                    // If we are cancelled we will just log
                    it.resume(getState(false)) { Logger.w("Resume while cancelled", it) }
                }
            }
        }
    }
}
