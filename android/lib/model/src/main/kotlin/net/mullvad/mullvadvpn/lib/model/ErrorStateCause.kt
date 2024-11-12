package net.mullvad.mullvadvpn.lib.model

import java.net.InetAddress

sealed class ErrorStateCause {
    class AuthFailed(private val error: AuthFailedError) : ErrorStateCause() {
        fun isCausedByExpiredAccount(): Boolean {
            return error is AuthFailedError.ExpiredAccount
        }
    }

    data object Ipv6Unavailable : ErrorStateCause()

    sealed class FirewallPolicyError : ErrorStateCause() {
        data object Generic : FirewallPolicyError()
    }

    data object DnsError : ErrorStateCause()

    // Regression
    data class InvalidDnsServers(val addresses: List<InetAddress>) : ErrorStateCause()

    data object StartTunnelError : ErrorStateCause()

    data class TunnelParameterError(val error: ParameterGenerationError) : ErrorStateCause()

    data object IsOffline : ErrorStateCause()

    data object VpnPermissionDenied : ErrorStateCause()
}

sealed interface AuthFailedError {
    data object ExpiredAccount : AuthFailedError

    data object InvalidAccount : AuthFailedError

    data object TooManyConnections : AuthFailedError

    data object Unknown : AuthFailedError
}
