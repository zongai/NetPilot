// User-facing error model (NP-058). No secrets in Message.

namespace NetPilot.Desktop.Models;

/// <summary>Stable error kinds aligned with docs/ERRORS.md / IPC ErrorBody.</summary>
public enum UserErrorKind
{
    None,
    InvalidInput,
    Unavailable,
    Timeout,
    Cancelled,
    PermissionDenied,
    FailedPrecondition,
    NotFound,
    Internal,
}

public sealed class UserError
{
    public UserErrorKind Kind { get; init; }
    public string Message { get; init; } = "";
    public string? Operation { get; init; }
    public int? Code { get; init; }

    public bool IsEmpty => Kind == UserErrorKind.None;

    public static UserError None { get; } = new() { Kind = UserErrorKind.None, Message = "" };

    public static UserError FromIpc(string? kind, string? message, string? operation = null, int? code = null)
    {
        var k = (kind ?? "internal").ToLowerInvariant() switch
        {
            "invalid_input" => UserErrorKind.InvalidInput,
            "unavailable" => UserErrorKind.Unavailable,
            "timeout" => UserErrorKind.Timeout,
            "cancelled" => UserErrorKind.Cancelled,
            "permission_denied" => UserErrorKind.PermissionDenied,
            "failed_precondition" => UserErrorKind.FailedPrecondition,
            "not_found" => UserErrorKind.NotFound,
            _ => UserErrorKind.Internal,
        };
        return new UserError
        {
            Kind = k,
            Message = string.IsNullOrWhiteSpace(message) ? k.ToString() : message!,
            Operation = operation,
            Code = code,
        };
    }

    public override string ToString() =>
        IsEmpty ? "" : $"{Kind}: {Message}";
}
