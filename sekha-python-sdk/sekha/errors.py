"""
Custom exceptions for Sekha SDK
"""


class SekhaError(Exception):
    """Base exception for all Sekha errors"""

    pass


class SekhaAPIError(SekhaError):
    """API returned an error response"""

    def __init__(self, message: str, status_code: int, response: str):
        self.message = message
        self.status_code = status_code
        self.response = response
        super().__init__(f"API Error {status_code}: {message}")


class SekhaNotFoundError(SekhaError):
    """Requested resource not found"""

    pass


class SekhaAuthError(SekhaError):
    """Authentication failed"""

    pass


class SekhaConnectionError(SekhaError):
    """Failed to connect to server"""

    pass


class SekhaValidationError(SekhaError):
    """Invalid input parameters"""

    pass


class SekhaRateLimitError(SekhaError):
    """Rate limit exceeded"""

    pass
