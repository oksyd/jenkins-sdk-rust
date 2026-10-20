import hudson.security.FullControlOnceLoggedInAuthorizationStrategy
import hudson.security.HudsonPrivateSecurityRealm
import hudson.security.csrf.DefaultCrumbIssuer
import jenkins.install.InstallState
import jenkins.model.Jenkins

def jenkins = Jenkins.get()
def realm = new HudsonPrivateSecurityRealm(false)
realm.createAccount("sdk-admin", "sdk-integration-password")
jenkins.setSecurityRealm(realm)
def authorization = new FullControlOnceLoggedInAuthorizationStrategy()
authorization.setAllowAnonymousRead(false)
jenkins.setAuthorizationStrategy(authorization)
jenkins.setCrumbIssuer(new DefaultCrumbIssuer(false))
jenkins.setNumExecutors(2)
jenkins.getUpdateCenter().getSites().clear()
jenkins.setInstallState(InstallState.INITIAL_SETUP_COMPLETED)
jenkins.save()
